//! PR snapshot classification boundaries are independently exercised in Rust and Verus.
#![allow(unexpected_cfgs)]

#[cfg(verus_keep_ghost)]
#[path = "../../pr/src/verified_snapshot.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_pr::verified_snapshot as policy;

#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[cfg_attr(not(verus_keep_ghost), test)]
fn snapshot_outcome_priority_is_proved() {
    let stale = policy::snapshot_outcome(true, true, true, true);
    let closed = policy::snapshot_outcome(false, true, true, true);
    let failed = policy::snapshot_outcome(false, false, true, true);
    let pending = policy::snapshot_outcome(false, false, false, true);
    let passed = policy::snapshot_outcome(false, false, false, false);
    assert(matches!(stale, policy::SnapshotOutcome::Stale));
    assert(matches!(closed, policy::SnapshotOutcome::Closed));
    assert(matches!(failed, policy::SnapshotOutcome::Failed));
    assert(matches!(pending, policy::SnapshotOutcome::Pending));
    assert(matches!(passed, policy::SnapshotOutcome::Passed));
}

}

#[cfg(not(verus_keep_ghost))]
#[test]
fn rust_snapshot_classification_covers_all_provider_states() {
    use chrono_pr::verified_snapshot::{
        CheckClass, SnapshotOutcome, check_class_label, classify_check, snapshot_outcome,
        snapshot_outcome_label,
    };

    for state in ["SUCCESS"] {
        assert_eq!(classify_check(state), CheckClass::Passed);
    }
    for state in [
        "MISSING",
        "QUEUED",
        "IN_PROGRESS",
        "WAITING",
        "REQUESTED",
        "PENDING",
        "EXPECTED",
    ] {
        assert_eq!(classify_check(state), CheckClass::Pending, "{state}");
    }
    for state in [
        "FAILURE",
        "ERROR",
        "CANCELLED",
        "TIMED_OUT",
        "ACTION_REQUIRED",
        "STARTUP_FAILURE",
        "STALE",
        "NEUTRAL",
        "SKIPPED",
    ] {
        assert_eq!(classify_check(state), CheckClass::Failed, "{state}");
    }
    for state in ["success", "unknown", ""] {
        assert_eq!(classify_check(state), CheckClass::Unknown, "{state:?}");
    }
    assert_eq!(check_class_label(CheckClass::Passed), "passed");
    assert_eq!(check_class_label(CheckClass::Pending), "pending");
    assert_eq!(check_class_label(CheckClass::Failed), "failed");
    assert_eq!(check_class_label(CheckClass::Unknown), "unknown");
    for (flags, expected) in [
        ((true, true, true, true), SnapshotOutcome::Stale),
        ((false, true, true, true), SnapshotOutcome::Closed),
        ((false, false, true, true), SnapshotOutcome::Failed),
        ((false, false, false, true), SnapshotOutcome::Pending),
        ((false, false, false, false), SnapshotOutcome::Passed),
    ] {
        assert_eq!(
            snapshot_outcome(flags.0, flags.1, flags.2, flags.3),
            expected
        );
    }
    assert_eq!(snapshot_outcome_label(SnapshotOutcome::Stale), "stale");
    assert_eq!(snapshot_outcome_label(SnapshotOutcome::Closed), "closed");
    assert_eq!(snapshot_outcome_label(SnapshotOutcome::Failed), "failed");
    assert_eq!(snapshot_outcome_label(SnapshotOutcome::Pending), "pending");
    assert_eq!(snapshot_outcome_label(SnapshotOutcome::Passed), "passed");
}
