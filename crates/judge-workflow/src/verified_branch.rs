//! Branch source-kind admission is a proved workflow policy.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), derive(Clone, Copy, Debug, PartialEq, Eq))]
pub enum SourceKind {
    Feature,
    Integration,
}

/// A source branch is admissible only when exactly one registered prefix
/// matches and the source is not the target branch.
pub open spec fn source_kind(
    feature_prefix_matches: bool,
    integration_prefix_matches: bool,
    is_target_branch: bool,
) -> Option<SourceKind> {
    if is_target_branch || feature_prefix_matches == integration_prefix_matches {
        None
    } else if feature_prefix_matches {
        Some(SourceKind::Feature)
    } else {
        Some(SourceKind::Integration)
    }
}

pub fn select_source_kind(
    feature_prefix_matches: bool,
    integration_prefix_matches: bool,
    is_target_branch: bool,
) -> (kind: Option<SourceKind>)
    ensures kind == source_kind(
        feature_prefix_matches, integration_prefix_matches, is_target_branch,
    ),
{
    if is_target_branch || feature_prefix_matches == integration_prefix_matches {
        None
    } else if feature_prefix_matches {
        Some(SourceKind::Feature)
    } else {
        Some(SourceKind::Integration)
    }
}

}
