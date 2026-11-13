//! libtest 清单判断的同源 Verus 定义。
#![allow(unexpected_cfgs)]

use std::collections::{BTreeMap, BTreeSet};

#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;

verus! {

pub open spec fn binary_hash_suffix(bytes: Seq<u8>) -> bool {
    bytes.len() > 0 && forall|i: int| 0 <= i < bytes.len() ==> (
        (48 <= #[trigger] bytes[i] && bytes[i] <= 57)
            || (97 <= bytes[i] && bytes[i] <= 102)
    )
}

/// Cargo 二进制后缀必须是非空的小写十六进制串。
pub fn valid_binary_hash_suffix(bytes: &[u8]) -> (valid: bool)
    ensures valid == binary_hash_suffix(bytes@),
{
    if bytes.is_empty() { return false; }
    let mut i = 0;
    while i < bytes.len()
        invariant 0 <= i <= bytes.len(), bytes.len() > 0,
            forall|j: int| 0 <= j < i ==> (
                (48 <= #[trigger] bytes@[j] && bytes@[j] <= 57)
                    || (97 <= bytes@[j] && bytes@[j] <= 102)
            ),
        decreases bytes.len() - i,
    {
        let byte = bytes[i];
        if !((48 <= byte && byte <= 57) || (97 <= byte && byte <= 102)) {
            return false;
        }
        i += 1;
    }
    true
}

pub open spec fn same_names(first: Seq<String>, second: Seq<String>) -> bool {
    first.len() == second.len()
        && forall|i: int| 0 <= i < first.len() ==> first[i]@ == second[i]@
}

pub fn same_name_sequence(first: &[String], second: &[String]) -> (same: bool)
    ensures same == same_names(first@, second@),
{
    if first.len() != second.len() { return false; }
    let mut i = 0;
    while i < first.len()
        invariant first.len() == second.len(), i <= first.len(),
            forall|j: int| 0 <= j < i ==> first@[j]@ == second@[j]@,
        decreases first.len() - i,
    {
        if !identifiers::equal_text(first[i].as_str(), second[i].as_str()) { return false; }
        i += 1;
    }
    true
}

pub open spec fn suite_counts_valid(
    tests: nat, benches: nat, names: nat, distinct_names: nat,
) -> bool {
    benches == 0 && tests == names && distinct_names == names
}

pub fn valid_suite_counts(
    tests: usize, benches: usize, names: usize, distinct_names: usize,
) -> (valid: bool)
    ensures valid == suite_counts_valid(tests as nat, benches as nat, names as nat, distinct_names as nat),
{
    benches == 0 && tests == names && distinct_names == names
}

pub open spec fn complete_listing(names_empty: bool, suites: nat, binaries: nat) -> bool {
    names_empty && suites == binaries
}

pub fn valid_complete_listing(names_empty: bool, suites: usize, binaries: usize) -> (valid: bool)
    ensures valid == complete_listing(names_empty, suites as nat, binaries as nat),
{
    names_empty && suites == binaries
}

}

verus! {

pub fn inventory_members_empty<K>(members: &BTreeSet<K>) -> (empty: bool)
    ensures empty == members@.is_empty(),
{
    members.is_empty()
}

pub open spec fn nonempty_inventory<K, V>(
    complete: Set<V>, groups: Map<K, BTreeSet<V>>,
) -> bool {
    !complete.is_empty() && !groups.dom().is_empty()
        && forall|key: K| #[trigger] groups.contains_key(key) ==> !groups[key]@.is_empty()
}

pub fn inventory_groups_nonempty<K: Ord, V>(
    complete: &BTreeSet<V>, groups: &BTreeMap<K, BTreeSet<V>>,
) -> (valid: bool)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures valid == nonempty_inventory(complete@, groups@),
{
    if complete.is_empty() || groups.is_empty() { return false; }
    let mut remaining = groups.iter();
    loop
        invariant
            vstd::laws_cmp::obeys_cmp::<K>(),
            !complete@.is_empty(), !groups@.dom().is_empty(),
            remaining@.1.to_set() == groups@.kv_pairs(),
            0 <= remaining@.0 <= remaining@.1.len(),
            forall|i: int| 0 <= i < remaining@.0
                ==> !remaining@.1[i].1@.is_empty(),
        decreases remaining@.1.len() - remaining@.0 + 1,
    {
        let (key, group) = match remaining.next() {
            Some(pair) => pair,
            None => {
                assert forall|key: K| #[trigger] groups@.contains_key(key)
                    implies !groups@[key]@.is_empty() by {
                    assert(groups@.kv_pairs().contains((key, groups@[key])));
                    assert(remaining@.1.to_set().contains((key, groups@[key])));
                    let at = choose|i: int| 0 <= i < remaining@.1.len()
                        && remaining@.1[i] == (key, groups@[key]);
                }
                return true;
            },
        };
        if group.is_empty() {
            assert(groups@.contains_pair(*key, *group));
            return false;
        }
    }
}

pub fn disjoint_inventory_members<K: Ord>(
    first: &BTreeSet<K>, second: &BTreeSet<K>,
) -> (disjoint: bool)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures disjoint == first@.disjoint(second@),
{
    broadcast use vstd::std_specs::btree::axiom_set_contains_deref_key;
    let mut remaining = second.iter();
    loop
        invariant
            vstd::laws_cmp::obeys_cmp::<K>(),
            remaining@.1.to_set() == second@,
            0 <= remaining@.0 <= remaining@.1.len(),
            forall|i: int| 0 <= i < remaining@.0
                ==> !first@.contains(remaining@.1[i]),
        decreases remaining@.1.len() - remaining@.0 + 1,
    {
        let item = match remaining.next() {
            Some(item) => item,
            None => {
                assert forall|item: K| #[trigger] second@.contains(item)
                    implies !first@.contains(item) by {
                    let at = choose|i: int| 0 <= i < remaining@.1.len()
                        && remaining@.1[i] == item;
                }
                return true;
            },
        };
        if first.contains(item) {
            assert(second@.contains(*item));
            return false;
        }
    }
}

/// 冲突时保持原累积集合；成功时恰好并入当前组，不丢失或制造名称。
pub fn merge_disjoint_inventory_group(
    union: &mut BTreeSet<String>, group: &BTreeSet<String>,
) -> (merged: bool)
    requires vstd::laws_cmp::obeys_cmp::<String>(),
    ensures
        merged == old(union)@.disjoint(group@),
        union@ == if merged { old(union)@.union(group@) } else { old(union)@ },
{
    if !disjoint_inventory_members(union, group) { return false; }
    let ghost initial = union@;
    let mut remaining = group.iter();
    loop
        invariant
            vstd::laws_cmp::obeys_cmp::<String>(),
            initial == old(union)@,
            initial.disjoint(group@),
            remaining@.1.to_set() == group@,
            0 <= remaining@.0 <= remaining@.1.len(),
            union@ == initial.union(remaining@.1.take(remaining@.0).to_set()),
        decreases remaining@.1.len() - remaining@.0 + 1,
    {
        let ghost previous_position = remaining@.0;
        let item = match remaining.next() {
            Some(item) => item,
            None => {
                assert(remaining@.1.take(remaining@.0) =~= remaining@.1);
                assert(union@ =~= initial.union(group@));
                return true;
            },
        };
        union.insert(item.clone());
        proof {
            remaining@.1.lemma_take_succ_push(previous_position);
            remaining@.1.take(previous_position).lemma_push_to_set_commute(*item);
            assert(remaining@.1.take(remaining@.0).to_set()
                =~= remaining@.1.take(previous_position).to_set().insert(*item));
            assert(union@ =~= initial.union(remaining@.1.take(remaining@.0).to_set()));
        }
    }
}

pub fn same_inventory_members<K: Ord>(
    first: &BTreeSet<K>, second: &BTreeSet<K>,
) -> (same: bool)
    requires vstd::laws_cmp::obeys_cmp::<K>(),
    ensures same == (first@ == second@),
{
    broadcast use vstd::std_specs::btree::axiom_set_contains_deref_key;
    if first.len() != second.len() { return false; }
    let mut remaining = first.iter();
    let _second_items = second.iter();
    assert(_second_items@.1.to_set() == second@);
    loop
        invariant
            vstd::laws_cmp::obeys_cmp::<K>(),
            first@.finite(), second@.finite(), first@.len() == second@.len(),
            remaining@.1.to_set() == first@,
            0 <= remaining@.0 <= remaining@.1.len(),
            forall|i: int| 0 <= i < remaining@.0
                ==> second@.contains(remaining@.1[i]),
        decreases remaining@.1.len() - remaining@.0 + 1,
    {
        let item = match remaining.next() {
            Some(item) => item,
            None => {
                assert forall|item: K| #[trigger] first@.contains(item)
                    implies second@.contains(item) by {
                    let at = choose|i: int| 0 <= i < remaining@.1.len()
                        && remaining@.1[i] == item;
                }
                proof { vstd::set_lib::lemma_subset_equality(first@, second@); }
                return true;
            },
        };
        if !second.contains(item) {
            assert(first@.contains(*item));
            return false;
        }
    }
}

}
