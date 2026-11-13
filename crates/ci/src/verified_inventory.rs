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

verus! {

pub enum InventoryPartitionDecision<'a> {
    Ignored,
    Empty,
    Overlap(&'a String),
    Difference(BTreeSet<String>),
    Complete,
}

pub open spec fn inventory_prefix_members(
    entries: Seq<(String, BTreeSet<String>)>, end: int,
) -> Set<String> {
    Set::new(|name: String| exists|i: int|
        0 <= i < end && #[trigger] entries[i].1@.contains(name))
}

pub open spec fn inventory_all_members(groups: Map<String, BTreeSet<String>>) -> Set<String> {
    Set::new(|name: String| exists|id: String|
        groups.contains_key(id) && #[trigger] groups[id]@.contains(name))
}

pub open spec fn inventory_overlapping_group(
    groups: Map<String, BTreeSet<String>>, id: String,
) -> bool {
    groups.contains_key(id) && exists|prior: String|
        groups.contains_key(prior) && prior != id
            && !groups[prior]@.disjoint(#[trigger] groups[id]@)
}

pub open spec fn inventory_groups_disjoint(groups: Map<String, BTreeSet<String>>) -> bool {
    forall|first: String, second: String|
        groups.contains_key(first) && groups.contains_key(second) && first != second
            ==> #[trigger] groups[first]@.disjoint(groups[second]@)
}

pub open spec fn inventory_prefix_disjoint(
    entries: Seq<(String, BTreeSet<String>)>, end: int,
) -> bool {
    forall|i: int| 0 <= i < end ==>
        inventory_prefix_members(entries, i).disjoint(#[trigger] entries[i].1@)
}

pub open spec fn inventory_ordered_groups(
    entries: Seq<(String, BTreeSet<String>)>, groups: Map<String, BTreeSet<String>>,
) -> bool {
    entries.to_set() == groups.kv_pairs()
        && entries.no_duplicates()
        && vstd::std_specs::btree::increasing_seq(
            entries.map(|i: int, pair: (String, BTreeSet<String>)| pair.0))
}

pub open spec fn inventory_partition_decision(
    complete: Set<String>, ignored: Set<String>, groups: Map<String, BTreeSet<String>>,
    decision: InventoryPartitionDecision,
) -> bool {
    match decision {
        InventoryPartitionDecision::Ignored => !ignored.is_empty(),
        InventoryPartitionDecision::Empty => ignored.is_empty() && !nonempty_inventory(complete, groups),
        InventoryPartitionDecision::Overlap(id) =>
            ignored.is_empty() && nonempty_inventory(complete, groups)
                && inventory_overlapping_group(groups, *id)
                && exists|entries: Seq<(String, BTreeSet<String>)>, i: int|
                    inventory_ordered_groups(entries, groups)
                        && 0 <= i < entries.len() && entries[i].0 == *id
                        && inventory_prefix_disjoint(entries, i)
                        && !inventory_prefix_members(entries, i).disjoint(entries[i].1@),
        InventoryPartitionDecision::Difference(union) =>
            ignored.is_empty() && nonempty_inventory(complete, groups)
                && union@ != complete && union@ == inventory_all_members(groups)
                && inventory_groups_disjoint(groups)
                && exists|entries: Seq<(String, BTreeSet<String>)>|
                    inventory_ordered_groups(entries, groups)
                        && inventory_prefix_disjoint(entries, entries.len() as int)
                        && union@ == inventory_prefix_members(entries, entries.len() as int),
        InventoryPartitionDecision::Complete =>
            ignored.is_empty() && nonempty_inventory(complete, groups)
                && complete == inventory_all_members(groups)
                && inventory_groups_disjoint(groups)
                && exists|entries: Seq<(String, BTreeSet<String>)>|
                    inventory_ordered_groups(entries, groups)
                        && inventory_prefix_disjoint(entries, entries.len() as int)
                        && complete == inventory_prefix_members(entries, entries.len() as int),
    }
}

proof fn inventory_all_members_from_entries(
    entries: Seq<(String, BTreeSet<String>)>, groups: Map<String, BTreeSet<String>>,
)
    requires entries.to_set() == groups.kv_pairs(),
    ensures inventory_prefix_members(entries, entries.len() as int) == inventory_all_members(groups),
{
    assert forall|name: String|
        inventory_prefix_members(entries, entries.len() as int).contains(name)
            <==> inventory_all_members(groups).contains(name) by {
        if inventory_prefix_members(entries, entries.len() as int).contains(name) {
            let at = choose|i: int| 0 <= i < entries.len() && entries[i].1@.contains(name);
            assert(entries.to_set().contains(entries[at]));
            assert(groups.contains_pair(entries[at].0, entries[at].1));
            assert(inventory_all_members(groups).contains(name));
        } else if inventory_all_members(groups).contains(name) {
            let id = choose|id: String| groups.contains_key(id) && groups[id]@.contains(name);
            assert(entries.to_set().contains((id, groups[id])));
            let at = choose|i: int| 0 <= i < entries.len() && entries[i] == (id, groups[id]);
            assert(inventory_prefix_members(entries, entries.len() as int).contains(name));
        }
    }
    assert(inventory_prefix_members(entries, entries.len() as int) =~= inventory_all_members(groups));
}

proof fn inventory_conflict_has_prior_group(
    entries: Seq<(String, BTreeSet<String>)>, groups: Map<String, BTreeSet<String>>, end: int,
)
    requires
        inventory_ordered_groups(entries, groups), 0 <= end < entries.len(),
        !inventory_prefix_members(entries, end).disjoint(entries[end].1@),
    ensures inventory_overlapping_group(groups, entries[end].0),
{
    let name = choose|name: String| inventory_prefix_members(entries, end).contains(name)
        && entries[end].1@.contains(name);
    let prior = choose|i: int| 0 <= i < end && entries[i].1@.contains(name);
    assert(entries.to_set().contains(entries[prior]));
    assert(entries.to_set().contains(entries[end]));
    assert(groups.contains_pair(entries[prior].0, entries[prior].1));
    assert(groups.contains_pair(entries[end].0, entries[end].1));
    if entries[prior].0 == entries[end].0 {
        assert(entries[prior].1 == entries[end].1);
        assert(entries[prior] == entries[end]);
        assert(false);
    }
    assert(!groups[entries[prior].0]@.disjoint(groups[entries[end].0]@));
}

proof fn inventory_disjoint_groups_from_entries(
    entries: Seq<(String, BTreeSet<String>)>, groups: Map<String, BTreeSet<String>>,
)
    requires inventory_ordered_groups(entries, groups),
        inventory_prefix_disjoint(entries, entries.len() as int),
    ensures inventory_groups_disjoint(groups),
{
    assert forall|first: String, second: String|
        groups.contains_key(first) && groups.contains_key(second) && first != second
            implies #[trigger] groups[first]@.disjoint(groups[second]@) by {
        assert(entries.to_set().contains((first, groups[first])));
        assert(entries.to_set().contains((second, groups[second])));
        let a = choose|i: int| 0 <= i < entries.len() && entries[i] == (first, groups[first]);
        let b = choose|i: int| 0 <= i < entries.len() && entries[i] == (second, groups[second]);
        assert(a != b);
        if !groups[first]@.disjoint(groups[second]@) {
            let name = choose|name: String| groups[first]@.contains(name) && groups[second]@.contains(name);
            if a < b {
                assert(inventory_prefix_members(entries, b).contains(name));
                assert(inventory_prefix_members(entries, b).disjoint(entries[b].1@));
            } else {
                assert(inventory_prefix_members(entries, a).contains(name));
                assert(inventory_prefix_members(entries, a).disjoint(entries[a].1@));
            }
            assert(false);
        }
    }
}

proof fn inventory_prefix_members_step(
    entries: Seq<(String, BTreeSet<String>)>, end: int,
)
    requires 0 <= end < entries.len(),
    ensures inventory_prefix_members(entries, end + 1)
        == inventory_prefix_members(entries, end).union(entries[end].1@),
{
    assert forall|name: String|
        inventory_prefix_members(entries, end + 1).contains(name)
            <==> inventory_prefix_members(entries, end).union(entries[end].1@).contains(name) by {
        if inventory_prefix_members(entries, end + 1).contains(name) {
            let at = choose|i: int| 0 <= i < end + 1 && entries[i].1@.contains(name);
            if at < end {
                assert(inventory_prefix_members(entries, end).contains(name));
            }
        } else if inventory_prefix_members(entries, end).contains(name) {
            let at = choose|i: int| 0 <= i < end && entries[i].1@.contains(name);
            assert(inventory_prefix_members(entries, end + 1).contains(name));
        } else if entries[end].1@.contains(name) {
            assert(inventory_prefix_members(entries, end + 1).contains(name));
        }
    }
    assert(inventory_prefix_members(entries, end + 1)
        =~= inventory_prefix_members(entries, end).union(entries[end].1@));
}

pub fn decide_inventory_partition<'a>(
    complete: &BTreeSet<String>, ignored: &BTreeSet<String>,
    groups: &'a BTreeMap<String, BTreeSet<String>>,
) -> (decision: InventoryPartitionDecision<'a>)
    requires vstd::laws_cmp::obeys_cmp::<String>(),
    ensures inventory_partition_decision(complete@, ignored@, groups@, decision),
{
    if !inventory_members_empty(ignored) { return InventoryPartitionDecision::Ignored; }
    if !inventory_groups_nonempty(complete, groups) { return InventoryPartitionDecision::Empty; }
    let mut union = BTreeSet::new();
    let mut remaining = groups.iter();
    let ghost entries = remaining@.1;
    assert(inventory_prefix_members(entries, 0) =~= Set::<String>::empty());
    loop
        invariant
            vstd::laws_cmp::obeys_cmp::<String>(),
            ignored@.is_empty(), nonempty_inventory(complete@, groups@),
            inventory_ordered_groups(entries, groups@),
            remaining@.1 == entries,
            0 <= remaining@.0 <= entries.len(),
            union@ == inventory_prefix_members(entries, remaining@.0),
            inventory_prefix_disjoint(entries, remaining@.0),
        decreases entries.len() - remaining@.0 + 1,
    {
        let ghost previous_position = remaining@.0;
        let (id, group) = match remaining.next() {
            Some(pair) => pair,
            None => {
                proof {
                    inventory_all_members_from_entries(entries, groups@);
                    inventory_disjoint_groups_from_entries(entries, groups@);
                }
                if !same_inventory_members(&union, complete) {
                    return InventoryPartitionDecision::Difference(union);
                }
                return InventoryPartitionDecision::Complete;
            },
        };
        if !merge_disjoint_inventory_group(&mut union, group) {
            assert(entries[previous_position].0 == *id);
            assert(!inventory_prefix_members(entries, previous_position).disjoint(
                entries[previous_position].1@));
            proof { inventory_conflict_has_prior_group(entries, groups@, previous_position); }
            return InventoryPartitionDecision::Overlap(id);
        }
        proof {
            inventory_prefix_members_step(entries, previous_position);
            assert forall|i: int| 0 <= i < remaining@.0 implies
                inventory_prefix_members(entries, i).disjoint(#[trigger] entries[i].1@) by {
                if i < previous_position {
                    assert(inventory_prefix_disjoint(entries, previous_position));
                }
            }
        }
    }
}

}

verus! {

pub open spec fn inventory_partition_valid(
    complete: Set<String>, ignored: Set<String>, groups: Map<String, BTreeSet<String>>,
) -> bool {
    ignored.is_empty() && nonempty_inventory(complete, groups)
        && inventory_groups_disjoint(groups) && inventory_all_members(groups) == complete
}

pub open spec fn inventory_partition_response<Format, Render>(
    decision: InventoryPartitionDecision, format_difference: Format, render: &Render,
    response: Result<(), String>,
) -> bool
    where Format: FnOnce(BTreeSet<String>) -> String, Render: Fn([&str; 2]) -> String,
{
    match (decision, response) {
        (InventoryPartitionDecision::Ignored, Err(message)) =>
            message@ == "ignored tests cannot establish complete coverage"@,
        (InventoryPartitionDecision::Empty, Err(message)) => message@ == "empty inventory/group"@,
        (InventoryPartitionDecision::Overlap(id), Err(message)) =>
            exists|detail: &str| detail@ == id@
                && render.ensures((["overlapping inventory group: ", detail],), message),
        (InventoryPartitionDecision::Difference(union), Err(message)) =>
            exists|suffix: String| format_difference.ensures((union,), suffix)
                && exists|detail: &str| detail@ == suffix@
                    && render.ensures((["inventory omission/addition: ", detail],), message),
        (InventoryPartitionDecision::Complete, Ok(())) => true,
        _ => false,
    }
}

pub fn inventory_partition_result<Format, Render>(
    decision: InventoryPartitionDecision, format_difference: Format, render: &Render,
) -> (response: Result<(), String>)
    where Format: FnOnce(BTreeSet<String>) -> String, Render: Fn([&str; 2]) -> String,
    requires forall|items: BTreeSet<String>| format_difference.requires((items,)),
        forall|parts: [&str; 2]| render.requires((parts,)),
    ensures inventory_partition_response(decision, format_difference, render, response),
{
    match decision {
        InventoryPartitionDecision::Ignored =>
            Err("ignored tests cannot establish complete coverage".to_owned()),
        InventoryPartitionDecision::Empty => Err("empty inventory/group".to_owned()),
        InventoryPartitionDecision::Overlap(id) => {
            Err(render(["overlapping inventory group: ", id.as_str()]))
        },
        InventoryPartitionDecision::Difference(union) => {
            let suffix = format_difference(union);
            Err(render(["inventory omission/addition: ", suffix.as_str()]))
        },
        InventoryPartitionDecision::Complete => Ok(()),
    }
}

pub fn check_inventory_partition<Format, Render>(
    complete: &BTreeSet<String>, ignored: &BTreeSet<String>,
    groups: &BTreeMap<String, BTreeSet<String>>, format_difference: Format, render: &Render,
) -> (response: Result<(), String>)
    where Format: FnOnce(BTreeSet<String>) -> String, Render: Fn([&str; 2]) -> String,
    requires vstd::laws_cmp::obeys_cmp::<String>(),
        forall|items: BTreeSet<String>| format_difference.requires((items,)),
        forall|parts: [&str; 2]| render.requires((parts,)),
    ensures
        response.is_ok() == inventory_partition_valid(complete@, ignored@, groups@),
        exists|decision: InventoryPartitionDecision|
            inventory_partition_decision(complete@, ignored@, groups@, decision)
                && inventory_partition_response(decision, format_difference, render, response),
{
    let decision = decide_inventory_partition(complete, ignored, groups);
    proof {
        if let InventoryPartitionDecision::Overlap(id) = decision {
            if inventory_groups_disjoint(groups@) {
                let prior = choose|prior: String| groups@.contains_key(prior) && prior != *id
                    && !groups@[prior]@.disjoint(groups@[*id]@);
                assert(false);
            }
        }
    }
    inventory_partition_result(decision, format_difference, render)
}

}
