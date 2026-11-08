//! 原件交接的请求边界、精确成员与局部释放；IO 仍归保留所有者。
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
#[path = "../../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use crate::verified_identifiers as identifiers;

verus! {

pub open spec fn present(values: Seq<String>, value: Seq<char>) -> bool {
    exists|at: int| 0 <= at < values.len() && (#[trigger] values[at])@ == value
}

pub fn contains(values: &[String], value: &str) -> (found: bool)
    ensures found == present(values@, value@),
{
    let mut at = 0;
    while at < values.len()
        invariant at <= values.len(),
            forall|before: int| 0 <= before < at ==> values@[before]@ != value@,
        decreases values.len() - at,
    {
        if identifiers::equal_text(values[at].as_str(), value) { return true; }
        at += 1;
    }
    false
}

pub open spec fn unique(values: Seq<String>) -> bool {
    forall|a: int, b: int| 0 <= a < b < values.len()
        ==> (#[trigger] values[a])@ != (#[trigger] values[b])@
}

pub fn unique_selection(values: &[String], maximum: usize) -> (valid: bool)
    ensures valid == (0 < values.len() <= maximum && unique(values@)),
{
    if values.len() == 0 || values.len() > maximum { return false; }
    let mut at = 0;
    while at < values.len()
        invariant at <= values.len(), unique(values@.take(at as int)),
        decreases values.len() - at,
    {
        let mut before = 0;
        while before < at
            invariant before <= at, at < values.len(), unique(values@.take(at as int)),
                forall|i: int| 0 <= i < before ==> values@[i]@ != values@[at as int]@,
            decreases at - before,
        {
            if identifiers::equal_text(values[before].as_str(), values[at].as_str()) {
                return false;
            }
            before += 1;
        }
        assert(unique(values@.take(at as int + 1)));
        at += 1;
    }
    assert(values@.take(at as int) =~= values@);
    true
}

pub open spec fn bounded_roles(values: Seq<String>) -> bool {
    forall|at: int| 0 <= at < values.len()
        ==> 0 < vstd::utf8::encode_utf8((#[trigger] values[at])@).len() <= 256
}

/// trim 和字节长度由标准字符串接口提供，不替换原 Unicode 空白语义。
pub fn valid_request(
    roles: &[String], maximum: usize, trimmed_reason: &str, reason_bytes: usize,
    released: bool, length: u64, max_length: u64,
) -> (valid: bool)
    ensures valid == (released && 0 < trimmed_reason@.len() && reason_bytes <= 4096
        && length <= max_length && 0 < roles.len() <= maximum
        && unique(roles@) && bounded_roles(roles@)),
{
    if !released || trimmed_reason.is_empty() || reason_bytes > 4096
        || length > max_length || !unique_selection(roles, maximum) { return false; }
    let mut at = 0;
    while at < roles.len()
        invariant at <= roles.len(),
            forall|i: int| 0 <= i < at
                ==> 0 < vstd::utf8::encode_utf8((#[trigger] roles@[i])@).len() <= 256,
        decreases roles.len() - at,
    {
        let bytes = roles[at].as_str().as_bytes();
        if bytes.len() == 0 || bytes.len() > 256 { return false; }
        at += 1;
    }
    true
}

pub fn same_members(left: &[String], right: &[String]) -> (same: bool)
    ensures same == (forall|value: Seq<char>| present(left@, value) == present(right@, value)),
{
    let mut at = 0;
    while at < left.len()
        invariant at <= left.len(),
            forall|i: int| 0 <= i < at ==> present(right@, left@[i]@),
        decreases left.len() - at,
    {
        if !contains(right, left[at].as_str()) { return false; }
        at += 1;
    }
    let mut at = 0;
    while at < right.len()
        invariant at <= right.len(),
            forall|i: int| 0 <= i < left.len() ==> present(right@, left@[i]@),
            forall|i: int| 0 <= i < at ==> present(left@, right@[i]@),
        decreases right.len() - at,
    {
        if !contains(left, right[at].as_str()) { return false; }
        at += 1;
    }
    true
}

pub open spec fn removed(values: Seq<String>, id: Seq<char>) -> Seq<String>
    decreases values.len(),
{
    if values.len() == 0 { Seq::empty() }
    else if values.last()@ == id { removed(values.drop_last(), id) }
    else { removed(values.drop_last(), id).push(values.last()) }
}

pub fn without(values: &[String], id: &str) -> (out: Vec<String>)
    ensures out@ == removed(values@, id@),
{
    let mut out = Vec::new();
    let mut at = 0;
    while at < values.len()
        invariant at <= values.len(), out@ == removed(values@.take(at as int), id@),
        decreases values.len() - at,
    {
        if !identifiers::equal_text(values[at].as_str(), id) { out.push(values[at].clone()); }
        proof {
            let next = values@.take(at as int + 1);
            assert(next.drop_last() =~= values@.take(at as int));
            assert(next.last() == values@[at as int]);
            reveal_with_fuel(removed, 1);
        }
        at += 1;
    }
    assert(values@.take(at as int) =~= values@);
    out
}

/// 所选原件的请求不进入全库事务、输出或消费者枚举。
pub fn global_maintenance(originals: Option<&[String]>) -> (global: bool)
    ensures global == originals.is_none(),
{
    match originals { None => true, Some(_) => false }
}

}
