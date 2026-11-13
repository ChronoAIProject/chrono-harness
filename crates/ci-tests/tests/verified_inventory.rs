//! 清单边界的独立预期，同时执行 Verus 证明和 Rust 断言。
#![allow(unexpected_cfgs)]

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
