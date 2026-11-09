//! Workflow evidence admission is a proved conjunction over adapted facts.
#![allow(unexpected_cfgs)]

#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// The JSON/process adapters may observe these facts, but this conjunction is
/// the sole owner of the evidence admission decision.
pub open spec fn record_admissible(
    state_executed: bool,
    judge_identity: bool,
    request_identity: bool,
    request_digest: bool,
    successful_exit: bool,
    protocol_status: bool,
    response_bytes: bool,
) -> bool {
    state_executed
        && judge_identity
        && request_identity
        && request_digest
        && successful_exit
        && protocol_status
        && response_bytes
}

/// Combine adapted response/process observations into the proved admission
/// result consumed by the workflow producer.
pub fn admit_record(
    state_executed: bool,
    judge_identity: bool,
    request_identity: bool,
    request_digest: bool,
    successful_exit: bool,
    protocol_status: bool,
    response_bytes: bool,
) -> (admitted: bool)
    ensures admitted == record_admissible(
        state_executed,
        judge_identity,
        request_identity,
        request_digest,
        successful_exit,
        protocol_status,
        response_bytes,
    ),
{
    state_executed
        && judge_identity
        && request_identity
        && request_digest
        && successful_exit
        && protocol_status
        && response_bytes
}

}
