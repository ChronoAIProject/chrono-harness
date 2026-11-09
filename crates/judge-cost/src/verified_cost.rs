//! Declared cost coordinate admission is a proved policy.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// A cost declaration is unknown when any of its four coordinates is absent.
pub open spec fn unknown_coordinate(
    cpu_missing: bool,
    wall_missing: bool,
    peak_rss_missing: bool,
    io_missing: bool,
) -> bool {
    cpu_missing || wall_missing || peak_rss_missing || io_missing
}

/// The production judge uses this result to emit the warning and preserve the
/// exact missing coordinate list in its JSON adapter.
pub fn has_unknown_coordinate(
    cpu_missing: bool,
    wall_missing: bool,
    peak_rss_missing: bool,
    io_missing: bool,
) -> (unknown: bool)
    ensures unknown == unknown_coordinate(
        cpu_missing, wall_missing, peak_rss_missing, io_missing,
    ),
{
    cpu_missing || wall_missing || peak_rss_missing || io_missing
}

}
