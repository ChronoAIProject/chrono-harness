//! 离线汇总实际使用的结果集合判断及精确 Verus 合同。
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_identifiers.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
use identifiers::equal_text;

verus! {

pub open spec fn text_member(values: Seq<&str>, target: Seq<char>) -> bool {
    exists|i: int| 0 <= i < values.len() && (#[trigger] values[i])@ == target
}

pub fn contains_text(values: &Vec<&str>, target: &str) -> (found: bool)
    ensures found <==> text_member(values@, target@),
{
    let mut i = 0;
    while i < values.len()
        invariant
            0 <= i <= values.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] values[j])@ != target@,
        decreases values.len() - i,
    {
        if equal_text(values[i], target) { return true; }
        i += 1;
    }
    false
}

pub open spec fn same_text_membership(actual: Seq<&str>, expected: Seq<&str>) -> bool {
    (forall|i: int| 0 <= i < actual.len() ==>
        text_member(expected, (#[trigger] actual[i])@))
        && (forall|i: int| 0 <= i < expected.len() ==>
            text_member(actual, (#[trigger] expected[i])@))
}

/// 不依赖顺序。重复 ID 的拒绝归响应协议，不由集合相等推导唯一性。
pub fn matches_text_membership(actual: &Vec<&str>, expected: &Vec<&str>) -> (matches: bool)
    ensures matches <==> same_text_membership(actual@, expected@),
{
    let mut i = 0;
    while i < actual.len()
        invariant 0 <= i <= actual.len(),
            forall|j: int| 0 <= j < i ==>
                text_member(expected@, (#[trigger] actual[j])@),
        decreases actual.len() - i,
    {
        if !contains_text(expected, actual[i]) { return false; }
        i += 1;
    }
    let mut i = 0;
    while i < expected.len()
        invariant 0 <= i <= expected.len(),
            forall|j: int| 0 <= j < actual.len() ==>
                text_member(expected@, (#[trigger] actual[j])@),
            forall|j: int| 0 <= j < i ==>
                text_member(actual@, (#[trigger] expected[j])@),
        decreases expected.len() - i,
    {
        if !contains_text(actual, expected[i]) { return false; }
        i += 1;
    }
    true
}

}
