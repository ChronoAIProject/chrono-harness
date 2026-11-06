//! 独立边界预期直接消费生产集合判断，Cargo 与 Verus 共用源码。
#![allow(unexpected_cfgs)]
#[path = "../../judge-ci/src/verified_collection.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn result_membership_preserves_permutations_and_rejects_missing_or_extra_ids() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("build"); reveal_strlit("test");
        reveal_strlit("other");
        assert("ci.inventory"@.len() != "build"@.len());
        assert("ci.inventory"@.len() != "test"@.len());
        assert("build"@[0] != "other"@[0]);
        assert("build"@.len() != "test"@.len());
        assert("test"@.len() != "other"@.len());
        assert("ci.inventory"@.len() != "other"@.len());
    }
    let expected = vec!["ci.inventory", "build", "test"];
    let reordered = vec!["test", "ci.inventory", "build"];
    let accepted = policy::matches_text_membership(&reordered, &expected);
    assert(accepted);
    let missing = vec!["ci.inventory", "build"];
    let absent = policy::matches_text_membership(&missing, &expected);
    assert(!absent);
    let extra = vec!["ci.inventory", "build", "test", "other"];
    let unexpected = policy::matches_text_membership(&extra, &expected);
    assert(!unexpected);
    let replaced = vec!["ci.inventory", "build", "other"];
    let changed = policy::matches_text_membership(&replaced, &expected);
    assert(!changed);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn result_membership_keeps_empty_and_duplicate_set_semantics() {
    proof { reveal_strlit("test"); }
    let empty: Vec<&str> = Vec::new();
    let same_empty = policy::matches_text_membership(&empty, &empty);
    assert(same_empty);
    let one = vec!["test"];
    let missing = policy::matches_text_membership(&empty, &one);
    assert(!missing);
    let extra = policy::matches_text_membership(&one, &empty);
    assert(!extra);
    let duplicates = vec!["test", "test"];
    let same_set = policy::matches_text_membership(&duplicates, &one);
    assert(same_set);
    let reverse = policy::matches_text_membership(&one, &duplicates);
    assert(reverse);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn result_membership_preserves_exact_unicode_and_nul_identity() {
    proof {
        reveal_strlit("é"); reveal_strlit("é"); reveal_strlit("a\0b");
        reveal_strlit("a");
        assert("é"@.len() != "é"@.len());
        assert("a\0b"@.len() != "a"@.len());
    }
    let composed = vec!["é"];
    let decomposed = vec!["é"];
    let unicode_changed = policy::matches_text_membership(&composed, &decomposed);
    assert(!unicode_changed);
    let nul = vec!["a\0b"];
    let prefix = vec!["a"];
    let truncated = policy::matches_text_membership(&nul, &prefix);
    assert(!truncated);
    let exact = policy::matches_text_membership(&nul, &nul);
    assert(exact);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registered_inventory_requires_the_marker_and_both_declared_lists() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("build"); reveal_strlit("test:other");
        assert("ci.inventory"@.len() != "build"@.len());
        assert("ci.inventory"@.len() != "test:other"@.len());
        assert("build"@.len() != "test:other"@.len());
    }
    let operations = vec!["build"];
    let not_required = vec!["test:other"];
    let reordered = vec!["test:other", "ci.inventory", "build"];
    let accepted = policy::matches_result_ids(&reordered, &operations, &not_required);
    assert(accepted);
    let no_inventory = vec!["build", "test:other"];
    let missing_marker = policy::matches_result_ids(&no_inventory, &operations, &not_required);
    assert(!missing_marker);
    let no_operation = vec!["ci.inventory", "test:other"];
    let missing_operation = policy::matches_result_ids(&no_operation, &operations, &not_required);
    assert(!missing_operation);
    let no_unrequired = vec!["ci.inventory", "build"];
    let missing_unrequired = policy::matches_result_ids(&no_unrequired, &operations, &not_required);
    assert(!missing_unrequired);
    let undeclared = policy::matches_result_ids(&reordered, &operations, &Vec::new());
    assert(!undeclared);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn registered_inventory_preserves_empty_lists_and_exact_declared_names() {
    proof {
        reveal_strlit("ci.inventory"); reveal_strlit("é"); reveal_strlit("a\0b");
        assert("ci.inventory"@.len() != "é"@.len());
        assert("ci.inventory"@.len() != "a\0b"@.len());
        assert("é"@.len() != "a\0b"@.len());
    }
    let empty: Vec<&str> = Vec::new();
    let marker = vec!["ci.inventory"];
    let inventory_only = policy::matches_result_ids(&marker, &empty, &empty);
    assert(inventory_only);
    let no_marker = policy::matches_result_ids(&empty, &empty, &empty);
    assert(!no_marker);
    let operations = vec!["é", "é"];
    let not_required = vec!["a\0b", "é"];
    let actual = vec!["a\0b", "ci.inventory", "é"];
    let exact_set = policy::matches_result_ids(&actual, &operations, &not_required);
    assert(exact_set);
}

}
