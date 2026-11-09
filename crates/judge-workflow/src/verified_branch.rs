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

/// A branch is fresh with respect to the registered commit-count limit when
/// its measured distance does not exceed that limit.
pub open spec fn behind_within_limit(behind: usize, max_behind: usize) -> bool {
    behind <= max_behind
}

/// The production branch adapter supplies the measured commit count and the
/// normalized registered limit; the decision itself is proved here.
pub fn commit_count_within_limit(behind: usize, max_behind: usize) -> (allowed: bool)
    ensures allowed == behind_within_limit(behind, max_behind),
{
    behind <= max_behind
}

/// An observed non-negative age is admissible when it is at or below the
/// normalized nanosecond threshold supplied by the external-input adapter.
pub open spec fn age_limit(age_ns: u64, max_age_ns: u64) -> bool {
    age_ns <= max_age_ns
}

/// The production adapter owns time parsing and threshold normalization; the
/// freshness decision is owned by this proved predicate.
pub fn age_within_limit(age_ns: u64, max_age_ns: u64) -> (allowed: bool)
    ensures allowed == age_limit(age_ns, max_age_ns),
{
    age_ns <= max_age_ns
}

}
