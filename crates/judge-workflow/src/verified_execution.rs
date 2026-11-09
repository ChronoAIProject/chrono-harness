//! Workflow execution completion admission is a proved conjunction over facts
//! adapted from the concrete plan and results records.
#![allow(unexpected_cfgs)]

#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

/// A collected execution is admissible only when every identity and shape
/// fact checked by the Rust adapter is present.
pub open spec fn collected_admissible(
    completion_schema: bool,
    completion_status: bool,
    collection_scope: bool,
    selected_identity: bool,
    no_executed_records: bool,
    no_blocked_records: bool,
    plan_identity: bool,
    result_selected_identity: bool,
    test_statuses: bool,
) -> bool {
    completion_schema
        && completion_status
        && collection_scope
        && selected_identity
        && no_executed_records
        && no_blocked_records
        && plan_identity
        && result_selected_identity
        && test_statuses
}

/// The adapter supplies the concrete comparisons; this function owns the
/// collected completion decision consumed by the workflow producer.
pub fn admit_collected(
    completion_schema: bool,
    completion_status: bool,
    collection_scope: bool,
    selected_identity: bool,
    no_executed_records: bool,
    no_blocked_records: bool,
    plan_identity: bool,
    result_selected_identity: bool,
    test_statuses: bool,
) -> (admitted: bool)
    ensures admitted == collected_admissible(
        completion_schema,
        completion_status,
        collection_scope,
        selected_identity,
        no_executed_records,
        no_blocked_records,
        plan_identity,
        result_selected_identity,
        test_statuses,
    ),
{
    completion_schema
        && completion_status
        && collection_scope
        && selected_identity
        && no_executed_records
        && no_blocked_records
        && plan_identity
        && result_selected_identity
        && test_statuses
}

/// A non-collected execution is admissible only when its plan, scope,
/// selected tests, statuses and operation count agree with the plan.
pub open spec fn execution_admissible(
    plan_identity: bool,
    scope_allowed: bool,
    selected_identity: bool,
    test_identity: bool,
    all_passed: bool,
    operation_count: bool,
) -> bool {
    plan_identity
        && scope_allowed
        && selected_identity
        && test_identity
        && all_passed
        && operation_count
}

/// Consume adapted execution facts through the proved ordinary-execution
/// decision. Per-operation receipt and process checks remain in their owners.
pub fn admit_execution(
    plan_identity: bool,
    scope_allowed: bool,
    selected_identity: bool,
    test_identity: bool,
    all_passed: bool,
    operation_count: bool,
) -> (admitted: bool)
    ensures admitted == execution_admissible(
        plan_identity,
        scope_allowed,
        selected_identity,
        test_identity,
        all_passed,
        operation_count,
    ),
{
    plan_identity
        && scope_allowed
        && selected_identity
        && test_identity
        && all_passed
        && operation_count
}

}
