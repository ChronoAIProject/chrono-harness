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

}
