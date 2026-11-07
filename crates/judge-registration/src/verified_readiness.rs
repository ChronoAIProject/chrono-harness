//! full 检查直接消费的登记状态与输入闭合判断。
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_identifiers.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
use identifiers::equal_text;

verus! {

pub open spec fn active(status: Option<&str>) -> bool {
    match status {
        Some(text) => text@ == "active"@,
        None => false,
    }
}

pub fn registry_active(status: Option<&str>) -> (ready: bool)
    ensures ready == active(status),
{
    match status {
        Some(text) => equal_text(text, "active"),
        None => false,
    }
}

pub open spec fn ready_inputs<T>(
    enforcement: Option<&str>, closure: Option<&str>, unresolved: Option<&Vec<T>>,
) -> bool {
    (match enforcement { Some(text) => text@ == "enabled"@, None => false })
        && (match closure { Some(text) => text@ == "declared-complete"@, None => false })
        && (match unresolved { Some(inputs) => inputs@.len() == 0, None => false })
}

pub fn input_ready<T>(
    enforcement: Option<&str>, closure: Option<&str>, unresolved: Option<&Vec<T>>,
) -> (ready: bool)
    ensures ready == ready_inputs(enforcement, closure, unresolved),
{
    (match enforcement { Some(text) => equal_text(text, "enabled"), None => false })
        && (match closure { Some(text) => equal_text(text, "declared-complete"), None => false })
        && (match unresolved { Some(inputs) => inputs.len() == 0, None => false })
}

}
