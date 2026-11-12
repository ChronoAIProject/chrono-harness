//! 执行原件的终态、完整身份和保留决策；实际 IO 由 state 所有者执行。
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;

verus! {

/// process_terminal 是现役 process_completion_known 对真实退出码／signal
/// 及全部自有工作加入结果的判断。
/// None 精确保留没有回执，与已返回但终态未知的 Some(false) 分开。
pub struct OperationObservation<'a> {
    pub status: &'a str,
    pub process_terminal: Option<bool>,
}

pub open spec fn operation_is_terminal(row: &OperationObservation) -> bool {
    row.status@ == "blocked"@ || row.process_terminal == Some(true)
}

pub fn terminal(row: &OperationObservation) -> (known: bool)
    ensures known == operation_is_terminal(row),
{
    identifiers::equal_text(row.status, "blocked")
        || match row.process_terminal { Some(value) => value, None => false }
}

pub fn operation_state(row: &OperationObservation) -> (state: &'static str)
    ensures state@ == if operation_is_terminal(row) { "completed"@ } else { "unknown"@ },
{
    if terminal(row) { "completed" } else { "unknown" }
}

pub fn all_terminal(rows: &Vec<OperationObservation>) -> (known: bool)
    ensures known == (forall|i: int| 0 <= i < rows.len() ==>
        operation_is_terminal(#[trigger] &rows[i])),
{
    let mut i = 0;
    while i < rows.len()
        invariant 0 <= i <= rows.len(),
            forall|j: int| 0 <= j < i ==> operation_is_terminal(#[trigger] &rows[j]),
        decreases rows.len() - i,
    {
        if !terminal(&rows[i]) { return false; }
        i += 1;
    }
    true
}

pub open spec fn original_matches(
    digest: Seq<char>, length: u64, expected: Option<(&str, u64)>,
) -> bool {
    match expected {
        None => true,
        Some((wanted_digest, wanted_length)) => digest == wanted_digest@ && length == wanted_length,
    }
}

pub fn validate_original(
    digest: &str, length: u64, expected: Option<(&str, u64)>,
) -> (out: Result<(), &'static str>)
    ensures out.is_ok() == original_matches(digest@, length, expected),
        match out {
            Err(message) => message@ == "operation original differs from the actual process result"@,
            Ok(()) => true,
        },
{
    match expected {
        None => Ok(()),
        Some((wanted_digest, wanted_length)) => {
            if identifiers::equal_text(digest, wanted_digest) && length == wanted_length { Ok(()) }
            else { Err("operation original differs from the actual process result") }
        },
    }
}

pub struct ReleasePlan {
    pub settle_inputs: bool,
    pub release_dependencies: bool,
    pub release_recovery: bool,
}

/// 输入须先交接，已知终态才可解除依赖与恢复保护；未知终态继续保护。
pub fn release_plan(known: bool, input_count: usize) -> (plan: ReleasePlan)
    ensures plan.settle_inputs == (input_count > 0),
        plan.release_dependencies == (known && input_count > 0),
        plan.release_recovery == known,
{
    ReleasePlan {
        settle_inputs: input_count > 0,
        release_dependencies: known && input_count > 0,
        release_recovery: known,
    }
}

}
