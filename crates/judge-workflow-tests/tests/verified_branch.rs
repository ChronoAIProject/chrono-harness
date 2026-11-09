//! Independent boundary expectations for branch source-kind admission.
#![allow(unexpected_cfgs)]
#[path = "../../judge-workflow/src/verified_branch.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn exactly_one_prefix_selects_feature_or_integration() {
    let feature = policy::select_source_kind(true, false, false);
    let integration = policy::select_source_kind(false, true, false);
    assert(matches!(feature, Some(policy::SourceKind::Feature)));
    assert(matches!(integration, Some(policy::SourceKind::Integration)));
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn zero_or_two_prefixes_are_rejected() {
    let none = policy::select_source_kind(false, false, false);
    let both = policy::select_source_kind(true, true, false);
    assert(none.is_none() && both.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn target_branch_is_rejected_even_with_one_prefix() {
    let feature = policy::select_source_kind(true, false, true);
    let integration = policy::select_source_kind(false, true, true);
    assert(feature.is_none() && integration.is_none());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn commit_count_limit_is_inclusive() {
    let zero = policy::commit_count_within_limit(0, 0);
    let exact = policy::commit_count_within_limit(7, 7);
    let over = policy::commit_count_within_limit(8, 7);
    assert(zero && exact && !over);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn commit_count_limit_preserves_large_unsigned_boundaries() {
    let largest = policy::commit_count_within_limit(usize::MAX, usize::MAX);
    let below = policy::commit_count_within_limit(usize::MAX - 1, usize::MAX);
    let over_zero = policy::commit_count_within_limit(1, 0);
    assert(largest && below && !over_zero);
}

}
