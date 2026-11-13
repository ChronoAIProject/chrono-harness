//! 清单边界的独立预期，同时执行 Verus 证明和 Rust 断言。
#![allow(unexpected_cfgs)]

use std::collections::{BTreeMap, BTreeSet};

#[cfg(verus_keep_ghost)]
#[path = "../../ci/src/verified_inventory.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_ci::verified_inventory as policy;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::string::StringSliceAdditionalSpecFns;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

fn expect_true(actual: bool)
    requires actual,
{
    if !actual { panic!(); }
}

#[cfg(not(verus_keep_ghost))]
#[test]
#[should_panic]
fn runtime_expectation_keeps_its_failure_branch() {
    expect_true(false);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn binary_suffix_accepts_only_nonempty_lower_hex() {
    expect_true(policy::valid_binary_hash_suffix(&[48u8, 49, 50, 57]));
    expect_true(policy::valid_binary_hash_suffix(&[97u8, 98, 99, 100, 101, 102]));
    expect_true(!policy::valid_binary_hash_suffix(&[]));
    expect_true(!policy::valid_binary_hash_suffix(&[65u8, 48]));
    expect_true(!policy::valid_binary_hash_suffix(&[97u8, 45, 103]));
    expect_true(!policy::valid_binary_hash_suffix(&[47u8]));
    expect_true(!policy::valid_binary_hash_suffix(&[58u8]));
    expect_true(!policy::valid_binary_hash_suffix(&[96u8]));
    expect_true(!policy::valid_binary_hash_suffix(&[103u8]));
    expect_true(!policy::valid_binary_hash_suffix(&[255u8]));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn ordered_names_preserve_length_duplicates_and_full_text() {
    proof { reveal_strlit("a"); reveal_strlit("b"); reveal_strlit("a\0"); }
    proof { assert("a"@[0] != "b"@[0]); }
    let first = ["a".to_owned(), "b".to_owned()];
    let same = ["a".to_owned(), "b".to_owned()];
    let changed = ["b".to_owned(), "a".to_owned()];
    let duplicate = ["a".to_owned(), "a".to_owned()];
    let short = ["a".to_owned()];
    let nul = ["a\0".to_owned(), "b".to_owned()];
    proof { assert(first@[0]@.len() == 1); assert(nul@[0]@.len() == 2); }
    expect_true(policy::same_name_sequence(&first, &same));
    expect_true(!policy::same_name_sequence(&first, &changed));
    expect_true(!policy::same_name_sequence(&first, &duplicate));
    expect_true(!policy::same_name_sequence(&first, &short));
    expect_true(!policy::same_name_sequence(&first, &nul));
    expect_true(policy::same_name_sequence(&duplicate, &duplicate));
    expect_true(policy::same_name_sequence(&[], &[]));
    let text = ["中\0\n".to_owned()];
    let same_text = ["中\0\n".to_owned()];
    expect_true(policy::same_name_sequence(&text, &same_text));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn suite_and_completion_counts_reject_incomplete_inventory() {
    expect_true(policy::valid_suite_counts(2, 0, 2, 2));
    expect_true(policy::valid_suite_counts(0, 0, 0, 0));
    expect_true(policy::valid_suite_counts(usize::MAX, 0, usize::MAX, usize::MAX));
    expect_true(!policy::valid_suite_counts(2, 1, 2, 2));
    expect_true(!policy::valid_suite_counts(1, 0, 2, 2));
    expect_true(!policy::valid_suite_counts(3, 0, 2, 2));
    expect_true(!policy::valid_suite_counts(2, 0, 2, 1));
    expect_true(!policy::valid_suite_counts(2, 0, 2, 3));
    expect_true(policy::valid_complete_listing(true, 3, 3));
    expect_true(policy::valid_complete_listing(true, 0, 0));
    expect_true(policy::valid_complete_listing(true, usize::MAX, usize::MAX));
    expect_true(!policy::valid_complete_listing(false, 3, 3));
    expect_true(!policy::valid_complete_listing(true, 2, 3));
    expect_true(!policy::valid_complete_listing(true, 4, 3));
}

}

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn inventory_set_equality_checks_members_not_only_counts() {
    let empty: BTreeSet<usize> = BTreeSet::new();
    let mut first = BTreeSet::new();
    first.insert(0usize);
    first.insert(usize::MAX);
    let mut same = BTreeSet::new();
    same.insert(usize::MAX);
    same.insert(0usize);
    let mut changed = BTreeSet::new();
    changed.insert(0usize);
    changed.insert(1usize);
    proof {
        assert(first@ =~= set![0usize, usize::MAX]);
        assert(same@ =~= set![0usize, usize::MAX]);
        assert(changed@ =~= set![0usize, 1usize]);
        assert(usize::MAX > 1);
        assert(first@.contains(usize::MAX));
        assert(!changed@.contains(usize::MAX));
        assert(first@ != changed@);
        assert(first@.contains(0usize) && same@.contains(0usize));
        assert(!first@.disjoint(same@));
    }
    expect_true(policy::inventory_members_empty(&empty));
    expect_true(!policy::inventory_members_empty(&first));
    expect_true(policy::same_inventory_members(&empty, &empty));
    expect_true(policy::same_inventory_members(&first, &same));
    expect_true(!policy::same_inventory_members(&first, &changed));
    expect_true(!policy::same_inventory_members(&first, &empty));
    expect_true(!policy::same_inventory_members(&empty, &first));
    expect_true(policy::disjoint_inventory_members(&first, &empty));
    expect_true(!policy::disjoint_inventory_members(&first, &same));
    let mut separate = BTreeSet::new();
    separate.insert(1usize);
    separate.insert(2usize);
    expect_true(policy::disjoint_inventory_members(&first, &separate));
    expect_true(policy::disjoint_inventory_members(&separate, &first));
    separate.insert(usize::MAX);
    expect_true(!policy::disjoint_inventory_members(&first, &separate));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn inventory_requires_complete_and_every_group_to_be_nonempty() {
    let empty: BTreeSet<usize> = BTreeSet::new();
    let mut complete = BTreeSet::new();
    complete.insert(7usize);
    let mut groups: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    expect_true(!policy::inventory_groups_nonempty(&complete, &groups));
    groups.insert(0, BTreeSet::new());
    expect_true(!policy::inventory_groups_nonempty(&complete, &groups));
    let mut first = BTreeSet::new();
    first.insert(7usize);
    groups.insert(0, first);
    expect_true(policy::inventory_groups_nonempty(&complete, &groups));
    expect_true(!policy::inventory_groups_nonempty(&empty, &groups));
    groups.insert(usize::MAX, BTreeSet::new());
    proof {
        assert(groups@.contains_key(usize::MAX));
        assert(groups@[usize::MAX]@.is_empty());
    }
    expect_true(!policy::inventory_groups_nonempty(&complete, &groups));
    let mut last = BTreeSet::new();
    last.insert(8usize);
    groups.insert(usize::MAX, last);
    expect_true(policy::inventory_groups_nonempty(&complete, &groups));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn inventory_group_merge_preserves_full_text_and_rejected_state()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let alpha = "a\0\n".to_owned();
    let beta = "中".to_owned();
    let gamma = "a".to_owned();
    proof {
        reveal_strlit("a\0\n"); reveal_strlit("中"); reveal_strlit("a");
        assert(alpha@.len() == 3);
        assert(beta@.len() == 1);
        assert(gamma@.len() == 1);
        assert(beta@[0] != gamma@[0]);
    }
    let mut union = BTreeSet::new();
    union.insert(alpha.clone());
    let empty = BTreeSet::new();
    expect_true(policy::merge_disjoint_inventory_group(&mut union, &empty));
    let mut group = BTreeSet::new();
    group.insert(beta.clone());
    group.insert(gamma.clone());
    expect_true(policy::merge_disjoint_inventory_group(&mut union, &group));
    let mut expected = BTreeSet::new();
    expected.insert(alpha.clone());
    expected.insert(beta.clone());
    expected.insert(gamma.clone());
    proof {
        assert(union@ =~= set![alpha, beta, gamma]);
        assert(expected@ =~= set![alpha, beta, gamma]);
    }
    expect_true(policy::same_inventory_members(&union, &expected));
    let before = union.clone();
    let mut conflict = BTreeSet::new();
    conflict.insert(alpha.clone());
    conflict.insert("unseen".to_owned());
    expect_true(!policy::merge_disjoint_inventory_group(&mut union, &conflict));
    expect_true(policy::same_inventory_members(&union, &before));
    let mut initially_empty = BTreeSet::new();
    expect_true(policy::merge_disjoint_inventory_group(&mut initially_empty, &group));
    proof { assert(initially_empty@ =~= group@); }
    expect_true(policy::same_inventory_members(&initially_empty, &group));
}

}

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn partition_rejects_ignored_before_empty_and_empty_before_coverage()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let empty: BTreeSet<String> = BTreeSet::new();
    let mut ignored = BTreeSet::new();
    ignored.insert("ignored\0中".to_owned());
    let mut groups: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    expect_true(match policy::decide_inventory_partition(&empty, &ignored, &groups) {
        policy::InventoryPartitionDecision::Ignored => true,
        _ => false,
    });
    expect_true(match policy::decide_inventory_partition(&empty, &empty, &groups) {
        policy::InventoryPartitionDecision::Empty => true,
        _ => false,
    });
    let mut complete = BTreeSet::new();
    complete.insert("one".to_owned());
    expect_true(match policy::decide_inventory_partition(&complete, &empty, &groups) {
        policy::InventoryPartitionDecision::Empty => true,
        _ => false,
    });
    groups.insert("empty\0中".to_owned(), BTreeSet::new());
    expect_true(match policy::decide_inventory_partition(&complete, &empty, &groups) {
        policy::InventoryPartitionDecision::Empty => true,
        _ => false,
    });
    expect_true(match policy::decide_inventory_partition(&complete, &ignored, &groups) {
        policy::InventoryPartitionDecision::Ignored => true,
        _ => false,
    });
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn partition_single_group_preserves_success_omission_addition_and_diagnostic_members()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let alpha = "a\0中".to_owned();
    let beta = "b".to_owned();
    let id = "group\0\n中".to_owned();
    proof {
        reveal_strlit("a\0中"); reveal_strlit("b");
        assert(alpha@.len() == 3); assert(beta@.len() == 1);
        assert(alpha != beta);
    }
    let ignored: BTreeSet<String> = BTreeSet::new();
    let mut group = BTreeSet::new(); group.insert(alpha.clone());
    let mut complete = BTreeSet::new(); complete.insert(alpha.clone());
    let mut groups = BTreeMap::new(); groups.insert(id.clone(), group);
    proof {
        assert(policy::nonempty_inventory(complete@, groups@));
        assert(policy::inventory_all_members(groups@) =~= complete@);
        assert forall|key: String| !policy::inventory_overlapping_group(groups@, key) by {
            if groups@.contains_key(key) { assert(key == id); }
        }
    }
    expect_true(match policy::decide_inventory_partition(&complete, &ignored, &groups) {
        policy::InventoryPartitionDecision::Complete => true,
        _ => false,
    });
    complete.insert(beta.clone());
    proof {
        assert(policy::inventory_all_members(groups@) =~= set![alpha]);
        assert(complete@ =~= set![alpha, beta]);
        assert(!policy::inventory_all_members(groups@).contains(beta));
    }
    match policy::decide_inventory_partition(&complete, &ignored, &groups) {
        policy::InventoryPartitionDecision::Difference(observed) => {
            expect_true(observed.len() == 1);
            expect_true(observed.contains(&alpha));
            expect_true(!observed.contains(&beta));
        },
        _ => expect_true(false),
    }
    let mut missing = BTreeSet::new(); missing.insert(beta.clone());
    match policy::decide_inventory_partition(&missing, &ignored, &groups) {
        policy::InventoryPartitionDecision::Difference(observed) => {
            expect_true(observed.len() == 1);
            expect_true(observed.contains(&alpha));
            expect_true(!observed.contains(&beta));
        },
        _ => expect_true(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn partition_overlap_precedes_coverage_and_later_empty_group_precedes_overlap()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    let name = "shared\0中".to_owned();
    let first = "a".to_owned();
    let second = "long\0中".to_owned();
    let ignored: BTreeSet<String> = BTreeSet::new();
    let unrelated = "unrelated".to_owned();
    let mut complete = BTreeSet::new(); complete.insert(unrelated.clone());
    let mut one = BTreeSet::new(); one.insert(name.clone());
    let mut two = BTreeSet::new(); two.insert(name.clone());
    let mut groups = BTreeMap::new(); groups.insert(first.clone(), one);
    groups.insert(second.clone(), two);
    proof {
        reveal_strlit("shared\0中"); reveal_strlit("unrelated");
        reveal_strlit("a"); reveal_strlit("long\0中");
        assert(first@.len() != second@.len()); assert(first != second);
        assert(name@.len() != "unrelated"@.len());
        assert(name != unrelated);
        assert(groups@[first]@.contains(name));
        assert(groups@[second]@.contains(name));
        assert forall|key: String| groups@.contains_key(key)
            implies groups@[key]@ == set![name] by {
            assert(key == first || key == second);
        }
        assert forall|member: String| policy::inventory_all_members(groups@).contains(member)
            <==> member == name by {
            if policy::inventory_all_members(groups@).contains(member) {
                let key = choose|key: String| groups@.contains_key(key) && groups@[key]@.contains(member);
                assert(groups@[key]@ == set![name]);
            } else if member == name {
                assert(groups@[first]@.contains(member));
            }
        }
        assert(policy::inventory_all_members(groups@) =~= set![name]);
        assert(!groups@[first]@.disjoint(groups@[second]@));
        assert(!policy::inventory_groups_disjoint(groups@));
        assert(policy::nonempty_inventory(complete@, groups@));
    }
    match policy::decide_inventory_partition(&complete, &ignored, &groups) {
        policy::InventoryPartitionDecision::Overlap(id) => {
            expect_true(policy::same_name_sequence(&[id.clone()], &[first.clone()])
                || policy::same_name_sequence(&[id.clone()], &[second.clone()]));
        },
        policy::InventoryPartitionDecision::Difference(_) => expect_true(false),
        _ => expect_true(false),
    }
    let empty_id = "empty".to_owned();
    groups.insert(empty_id.clone(), BTreeSet::new());
    proof {
        assert(groups@.contains_key(empty_id));
        assert(groups@[empty_id]@.is_empty());
        assert(!policy::nonempty_inventory(complete@, groups@));
    }
    expect_true(match policy::decide_inventory_partition(&complete, &ignored, &groups) {
        policy::InventoryPartitionDecision::Empty => true,
        _ => false,
    });
}

}

verus! {

fn render_partition_detail(parts: [&str; 2]) -> (text: String)
    ensures text@ == parts[1]@,
{
    parts[1].to_owned()
}

fn format_partition_difference(_items: BTreeSet<String>) -> (text: String)
    ensures text@ == "difference\0中\n"@,
{
    "difference\0中\n".to_owned()
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn partition_response_keeps_fixed_errors_full_payload_and_success()
    requires vstd::laws_cmp::obeys_cmp::<String>(),
{
    match policy::inventory_partition_result(policy::InventoryPartitionDecision::Ignored,
        format_partition_difference, &render_partition_detail) {
        Err(message) => expect_true(policy::same_name_sequence(&[message],
            &["ignored tests cannot establish complete coverage".to_owned()])),
        Ok(()) => expect_true(false),
    }
    match policy::inventory_partition_result(policy::InventoryPartitionDecision::Empty,
        format_partition_difference, &render_partition_detail) {
        Err(message) => expect_true(policy::same_name_sequence(&[message],
            &["empty inventory/group".to_owned()])),
        Ok(()) => expect_true(false),
    }
    let id = "group\0中\n".to_owned();
    match policy::inventory_partition_result(policy::InventoryPartitionDecision::Overlap(&id),
        format_partition_difference, &render_partition_detail) {
        Err(message) => expect_true(policy::same_name_sequence(&[message], &[id.clone()])),
        Ok(()) => expect_true(false),
    }
    match policy::inventory_partition_result(policy::InventoryPartitionDecision::Difference(BTreeSet::new()),
        format_partition_difference, &render_partition_detail) {
        Err(message) => expect_true(policy::same_name_sequence(&[message], &["difference\0中\n".to_owned()])),
        Ok(()) => expect_true(false),
    }
    expect_true(policy::inventory_partition_result(policy::InventoryPartitionDecision::Complete,
        format_partition_difference, &render_partition_detail).is_ok());
    let ignored: BTreeSet<String> = BTreeSet::new();
    let mut complete = BTreeSet::new(); complete.insert(id.clone());
    let mut group = BTreeSet::new(); group.insert(id.clone());
    let mut groups = BTreeMap::new(); groups.insert("only".to_owned(), group);
    proof {
        assert(policy::nonempty_inventory(complete@, groups@));
        assert(policy::inventory_groups_disjoint(groups@));
        assert(policy::inventory_all_members(groups@) =~= complete@);
    }
    expect_true(policy::check_inventory_partition(&complete, &ignored, &groups,
        format_partition_difference, &render_partition_detail).is_ok());
    expect_true(policy::check_inventory_partition(&ignored, &ignored, &groups,
        format_partition_difference, &render_partition_detail).is_err());
}

}
