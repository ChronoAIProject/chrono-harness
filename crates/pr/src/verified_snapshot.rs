//! Pull request check and snapshot outcome classification with a single Verus owner.
#![allow(unexpected_cfgs)]

#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
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

#[cfg_attr(not(verus_keep_ghost), derive(Clone, Copy, Debug, PartialEq, Eq))]
pub enum CheckClass {
    Passed,
    Pending,
    Failed,
    Unknown,
}

#[cfg_attr(not(verus_keep_ghost), derive(Clone, Copy, Debug, PartialEq, Eq))]
pub enum SnapshotOutcome {
    Stale,
    Closed,
    Failed,
    Pending,
    Passed,
}

pub open spec fn check_class_spec(state: Seq<char>) -> CheckClass {
    if state == "SUCCESS"@ {
        CheckClass::Passed
    } else if state == "MISSING"@ || state == "QUEUED"@
        || state == "IN_PROGRESS"@ || state == "WAITING"@
        || state == "REQUESTED"@ || state == "PENDING"@ || state == "EXPECTED"@ {
        CheckClass::Pending
    } else if state == "FAILURE"@ || state == "ERROR"@
        || state == "CANCELLED"@ || state == "TIMED_OUT"@
        || state == "ACTION_REQUIRED"@ || state == "STARTUP_FAILURE"@
        || state == "STALE"@ || state == "NEUTRAL"@ || state == "SKIPPED"@ {
        CheckClass::Failed
    } else {
        CheckClass::Unknown
    }
}

pub fn classify_check(state: &str) -> (class: CheckClass)
    ensures class == check_class_spec(state@),
{
    if identifiers::equal_text(state, "SUCCESS") {
        CheckClass::Passed
    } else if identifiers::equal_text(state, "MISSING")
        || identifiers::equal_text(state, "QUEUED")
        || identifiers::equal_text(state, "IN_PROGRESS")
        || identifiers::equal_text(state, "WAITING")
        || identifiers::equal_text(state, "REQUESTED")
        || identifiers::equal_text(state, "PENDING")
        || identifiers::equal_text(state, "EXPECTED") {
        CheckClass::Pending
    } else if identifiers::equal_text(state, "FAILURE")
        || identifiers::equal_text(state, "ERROR")
        || identifiers::equal_text(state, "CANCELLED")
        || identifiers::equal_text(state, "TIMED_OUT")
        || identifiers::equal_text(state, "ACTION_REQUIRED")
        || identifiers::equal_text(state, "STARTUP_FAILURE")
        || identifiers::equal_text(state, "STALE")
        || identifiers::equal_text(state, "NEUTRAL")
        || identifiers::equal_text(state, "SKIPPED") {
        CheckClass::Failed
    } else {
        CheckClass::Unknown
    }
}

pub fn check_class_label(class: CheckClass) -> (label: &'static str)
    ensures label@ == match class {
        CheckClass::Passed => "passed"@,
        CheckClass::Pending => "pending"@,
        CheckClass::Failed => "failed"@,
        CheckClass::Unknown => "unknown"@,
    },
{
    match class {
        CheckClass::Passed => "passed",
        CheckClass::Pending => "pending",
        CheckClass::Failed => "failed",
        CheckClass::Unknown => "unknown",
    }
}

/// PR 快照只接受 provider 返回的完整小写 40 位提交 SHA。
#[verifier::inline]
pub open spec fn sha_byte(byte: u8) -> bool {
    48 <= byte <= 57 || 97 <= byte <= 102
}

pub open spec fn sha_format(bytes: Seq<u8>) -> bool {
    bytes.len() == 40
        && forall|i: int| 0 <= i < bytes.len() ==> sha_byte(#[trigger] bytes[i])
}

pub fn valid_sha(bytes: &[u8]) -> (valid: bool)
    ensures valid == sha_format(bytes@),
{
    if bytes.len() != 40 {
        return false;
    }
    let mut i = 0;
    while i < bytes.len()
        invariant
            0 <= i <= bytes.len(),
            bytes.len() == 40,
            forall|j: int| 0 <= j < i ==> sha_byte(#[trigger] bytes[j]),
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

pub fn valid_sha_text(text: &str) -> (valid: bool)
    ensures valid == sha_format(text.spec_bytes()),
{
    valid_sha(text.as_bytes())
}

pub open spec fn pr_state_spec(state: Seq<char>) -> bool {
    state == "OPEN"@ || state == "CLOSED"@ || state == "MERGED"@
}

pub fn valid_pr_state(state: &str) -> (valid: bool)
    ensures valid == pr_state_spec(state@),
{
    identifiers::equal_text(state, "OPEN")
        || identifiers::equal_text(state, "CLOSED")
        || identifiers::equal_text(state, "MERGED")
}

pub open spec fn snapshot_outcome_spec(
    stale: bool, closed: bool, failed: bool, pending: bool,
) -> SnapshotOutcome {
    if stale {
        SnapshotOutcome::Stale
    } else if closed {
        SnapshotOutcome::Closed
    } else if failed {
        SnapshotOutcome::Failed
    } else if pending {
        SnapshotOutcome::Pending
    } else {
        SnapshotOutcome::Passed
    }
}

pub fn observed_head_is_stale(observed: &str, expected: &str) -> (stale: bool)
    ensures stale == (observed@ != expected@),
{ !identifiers::equal_text(observed, expected) }

pub fn pull_request_closed(state: &str) -> (closed: bool)
    ensures closed == (state@ == "CLOSED"@),
{ identifiers::equal_text(state, "CLOSED") }

pub fn pull_request_open(state: &str) -> (open: bool)
    ensures open == (state@ == "OPEN"@),
{ identifiers::equal_text(state, "OPEN") }

pub fn pull_request_merged(state: &str) -> (merged: bool)
    ensures merged == (state@ == "MERGED"@),
{ identifiers::equal_text(state, "MERGED") }

pub fn snapshot_outcome(stale: bool, closed: bool, failed: bool, pending: bool)
    -> (outcome: SnapshotOutcome)
    ensures outcome == snapshot_outcome_spec(stale, closed, failed, pending),
{
    if stale {
        SnapshotOutcome::Stale
    } else if closed {
        SnapshotOutcome::Closed
    } else if failed {
        SnapshotOutcome::Failed
    } else if pending {
        SnapshotOutcome::Pending
    } else {
        SnapshotOutcome::Passed
    }
}

pub fn snapshot_outcome_label(outcome: SnapshotOutcome) -> (label: &'static str)
    ensures label@ == match outcome {
        SnapshotOutcome::Stale => "stale"@,
        SnapshotOutcome::Closed => "closed"@,
        SnapshotOutcome::Failed => "failed"@,
        SnapshotOutcome::Pending => "pending"@,
        SnapshotOutcome::Passed => "passed"@,
    },
{
    match outcome {
        SnapshotOutcome::Stale => "stale",
        SnapshotOutcome::Closed => "closed",
        SnapshotOutcome::Failed => "failed",
        SnapshotOutcome::Pending => "pending",
        SnapshotOutcome::Passed => "passed",
    }
}

}
