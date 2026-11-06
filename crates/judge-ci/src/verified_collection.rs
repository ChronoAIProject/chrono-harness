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

pub open spec fn text_values(values: Seq<&str>) -> Seq<Seq<char>> {
    values.map(|i: int, text: &str| text@)
}

pub open spec fn character_member(values: Seq<Seq<char>>, target: Seq<char>) -> bool {
    exists|i: int| 0 <= i < values.len() && (#[trigger] values[i]) == target
}

proof fn text_member_representation(values: Seq<&str>, target: Seq<char>)
    ensures text_member(values, target) <==> character_member(text_values(values), target),
{
    if text_member(values, target) {
        let i = choose|i: int| 0 <= i < values.len() && (#[trigger] values[i])@ == target;
        assert(text_values(values)[i] == target);
    }
    if character_member(text_values(values), target) {
        let i = choose|i: int| 0 <= i < text_values(values).len()
            && text_values(values)[i] == target;
        assert(values[i]@ == target);
    }
}

pub open spec fn result_membership(actual: Seq<&str>, expected: Seq<Seq<char>>) -> bool {
    (forall|i: int| 0 <= i < actual.len() ==>
        character_member(expected, (#[trigger] actual[i])@))
        && (forall|i: int| 0 <= i < expected.len() ==>
            text_member(actual, #[trigger] expected[i]))
}

proof fn result_membership_representation(actual: Seq<&str>, expected: Seq<&str>)
    ensures same_text_membership(actual, expected)
        <==> result_membership(actual, text_values(expected)),
{
    if same_text_membership(actual, expected) {
        assert forall|i: int| 0 <= i < actual.len() implies
            character_member(text_values(expected), (#[trigger] actual[i])@) by {
                text_member_representation(expected, actual[i]@);
            }
        assert forall|i: int| 0 <= i < text_values(expected).len() implies
            text_member(actual, #[trigger] text_values(expected)[i]) by {
                assert(text_values(expected)[i] == expected[i]@);
                assert(text_member(actual, expected[i]@));
            }
    }
    if result_membership(actual, text_values(expected)) {
        assert forall|i: int| 0 <= i < actual.len() implies
            text_member(expected, (#[trigger] actual[i])@) by {
                text_member_representation(expected, actual[i]@);
            }
        assert forall|i: int| 0 <= i < expected.len() implies
            text_member(actual, (#[trigger] expected[i])@) by {
                assert(text_values(expected)[i] == expected[i]@);
                assert(text_member(actual, text_values(expected)[i]));
            }
    }
}

pub open spec fn expected_result_ids(
    operations: Seq<&str>, not_required: Seq<&str>,
) -> Seq<Seq<char>> {
    seq!["ci.inventory"@] + text_values(operations) + text_values(not_required)
}

/// 期望结果恰为库存、登记操作和未要求测试；不从实际结果补期望。
pub fn matches_result_ids(
    actual: &Vec<&str>, operations: &Vec<&str>, not_required: &Vec<&str>,
) -> (matches: bool)
    ensures matches <==> result_membership(
        actual@, expected_result_ids(operations@, not_required@)),
{
    let mut expected = Vec::new();
    expected.push("ci.inventory");
    let mut i = 0;
    while i < operations.len()
        invariant 0 <= i <= operations.len(),
            text_values(expected@) == seq!["ci.inventory"@] + text_values(operations@).take(i as int),
        decreases operations.len() - i,
    {
        let ghost before = text_values(expected@);
        expected.push(operations[i]);
        proof {
            assert(text_values(expected@) =~= before.push(operations@[i as int]@));
            assert(text_values(operations@).take(i + 1)
                =~= text_values(operations@).take(i as int).push(operations@[i as int]@));
        }
        i += 1;
    }
    let mut i = 0;
    while i < not_required.len()
        invariant 0 <= i <= not_required.len(),
            text_values(expected@) == seq!["ci.inventory"@] + text_values(operations@)
                + text_values(not_required@).take(i as int),
        decreases not_required.len() - i,
    {
        let ghost before = text_values(expected@);
        expected.push(not_required[i]);
        proof {
            assert(text_values(expected@) =~= before.push(not_required@[i as int]@));
            assert(text_values(not_required@).take(i + 1)
                =~= text_values(not_required@).take(i as int).push(not_required@[i as int]@));
        }
        i += 1;
    }
    let matches = matches_text_membership(actual, &expected);
    proof {
        assert(text_values(expected@) =~= expected_result_ids(operations@, not_required@));
        result_membership_representation(actual@, expected@);
    }
    matches
}

}
