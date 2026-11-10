//! 独立边界预期消费真实状态函数，并作为普通 Rust 测试运行。
#![allow(unexpected_cfgs)]
#[path = "../../judge-projects/src/verified_state.rs"]
mod state;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;

verus! {

#[cfg(verus_keep_ghost)]
fn any_terminal_process_is_known_without_a_status_success_claim(status: &str) {
    let row = state::OperationObservation { status, process_terminal: Some(true) };
    let known = state::terminal(&row);
    assert(known);
}

#[cfg(verus_keep_ghost)]
fn any_other_status_needs_a_terminal_process(status: &str)
    requires status@ != "blocked"@,
{
    let absent = state::OperationObservation { status, process_terminal: None };
    let unknown = state::OperationObservation { status, process_terminal: Some(false) };
    let absent_known = state::terminal(&absent);
    let unknown_known = state::terminal(&unknown);
    assert(!absent_known && !unknown_known);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn blocked_is_terminal_without_a_process_and_unknown_receipts_stay_unknown() {
    proof {
        reveal_strlit("blocked"); reveal_strlit("passed");
        assert("blocked"@.len() != "passed"@.len());
    }
    let blocked = state::OperationObservation { status: "blocked", process_terminal: None };
    let blocked_known = state::terminal(&blocked);
    assert(blocked_known);
    let completed = state::operation_state(&blocked);
    let exact_completed = identifiers::equal_text(completed, "completed");
    assert(exact_completed);
    let claimed_success = state::OperationObservation { status: "passed", process_terminal: None };
    let no_receipt = state::terminal(&claimed_success);
    assert(!no_receipt);
    let returned_unknown = state::OperationObservation { status: "passed", process_terminal: Some(false) };
    let unknown = state::operation_state(&returned_unknown);
    let exact_unknown = identifiers::equal_text(unknown, "unknown");
    assert(exact_unknown);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn status_identity_preserves_case_suffix_nul_and_unicode() {
    proof {
        reveal_strlit("blocked"); reveal_strlit("Blocked"); reveal_strlit("blocked\0");
        reveal_strlit("blocked "); reveal_strlit("阻断");
        assert("blocked"@[0] != "Blocked"@[0]);
        assert("blocked"@.len() != "blocked\0"@.len());
        assert("blocked"@.len() != "blocked "@.len());
        assert("blocked"@.len() != "阻断"@.len());
    }
    let case = state::terminal(&state::OperationObservation { status: "Blocked", process_terminal: None });
    assert(!case);
    let nul = state::terminal(&state::OperationObservation { status: "blocked\0", process_terminal: None });
    assert(!nul);
    let suffix = state::terminal(&state::OperationObservation { status: "blocked ", process_terminal: None });
    assert(!suffix);
    let unicode = state::terminal(&state::OperationObservation { status: "阻断", process_terminal: None });
    assert(!unicode);
    let process = state::terminal(&state::OperationObservation { status: "阻断", process_terminal: Some(true) });
    assert(process);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_execution_needs_every_operation_and_accepts_the_empty_execution() {
    proof {
        reveal_strlit("blocked"); reveal_strlit("error");
        assert("blocked"@.len() != "error"@.len());
    }
    let empty = Vec::new();
    let empty_known = state::all_terminal(&empty);
    assert(empty_known);
    let mut rows = Vec::new();
    rows.push(state::OperationObservation { status: "blocked", process_terminal: None });
    rows.push(state::OperationObservation { status: "error", process_terminal: Some(true) });
    let returned = state::all_terminal(&rows);
    assert(returned);
    rows.push(state::OperationObservation { status: "error", process_terminal: Some(false) });
    let unresolved_last = state::all_terminal(&rows);
    assert(!unresolved_last);
    let mut unresolved_first = Vec::new();
    unresolved_first.push(state::OperationObservation { status: "error", process_terminal: None });
    unresolved_first.push(state::OperationObservation { status: "blocked", process_terminal: None });
    let incomplete = state::all_terminal(&unresolved_first);
    assert(!incomplete);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn original_identity_requires_the_complete_digest_and_exact_length() {
    proof {
        reveal_strlit("aé\0"); reveal_strlit("aé"); reveal_strlit("Aé\0");
        assert("aé\0"@.len() != "aé"@.len());
        assert("aé\0"@[0] != "Aé\0"@[0]);
    }
    let same = state::validate_original("aé\0", 7, Some(("aé\0", 7)));
    assert(same.is_ok());
    let length = state::validate_original("aé\0", 8, Some(("aé\0", 7)));
    assert(length.is_err());
    let suffix = state::validate_original("aé", 7, Some(("aé\0", 7)));
    assert(suffix.is_err());
    let case = state::validate_original("Aé\0", 7, Some(("aé\0", 7)));
    assert(case.is_err());
    match case {
        Err(message) => {
            let exact = identifiers::equal_text(message, "operation original differs from the actual process result");
            assert(exact);
        },
        Ok(()) => { assert(false); },
    }
    let unbound = state::validate_original("aé\0", u64::MAX, None);
    assert(unbound.is_ok());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn unknown_results_keep_recovery_and_known_results_release_after_input_handoff() {
    let unknown_inputs = state::release_plan(false, 3);
    assert(unknown_inputs.settle_inputs);
    assert(!unknown_inputs.release_dependencies && !unknown_inputs.release_recovery);
    let unknown_empty = state::release_plan(false, 0);
    assert(!unknown_empty.settle_inputs && !unknown_empty.release_dependencies && !unknown_empty.release_recovery);
    let known_inputs = state::release_plan(true, usize::MAX);
    assert(known_inputs.settle_inputs && known_inputs.release_dependencies && known_inputs.release_recovery);
    let known_empty = state::release_plan(true, 0);
    assert(!known_empty.settle_inputs && !known_empty.release_dependencies && known_empty.release_recovery);
}

}
