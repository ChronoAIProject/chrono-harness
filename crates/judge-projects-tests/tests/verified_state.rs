//! 独立边界预期消费真实状态函数，并作为普通 Rust 测试运行。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "support/host.rs"]
mod project_host;
#[cfg(verus_keep_ghost)]
#[path = "execution.rs"]
mod scheduler;
#[path = "../../judge-projects/src/verified_state.rs"]
mod state;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;

#[cfg(verus_keep_ghost)]
#[path = "../../judge-routes-tests/tests/support/tool.rs"]
mod compiler_fixture;

verus! {

#[cfg(verus_keep_ghost)]
fn successful_execution_may_still_need_its_trace() {
    assert(scheduler::failure_batch::keep_scheduler_diagnostics(true, false));
    assert(scheduler::failure_batch::keep_scheduler_diagnostics(false, true));
    assert(!scheduler::failure_batch::keep_scheduler_diagnostics(true, true));
    assert(project_host::failed_command(false, Some(7u8)).is_none());
    assert(project_host::failed_command(true, Some(7u8)) == Some(7u8));
    assert(project_host::failed_command::<u8>(true, None).is_none());
}


#[cfg(verus_keep_ghost)]
fn a_mismatch_between_the_two_compiler_destinations_is_rejected(count: usize, has_flag: bool) {
    let accepted = compiler_fixture::accepts_output(count, has_flag, false);
    assert(!accepted);
}

#[cfg(verus_keep_ghost)]
fn another_operations_compiler_output_cannot_be_claimed(flat: bool, canonical: bool) {
    let owned = state::operation_io::owned_compiler_output(false, flat, canonical);
    assert(!owned);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn compiler_release_needs_success_join_and_operation_ownership() {
    proof {
        reveal_strlit("passed"); reveal_strlit("error");
        assert("passed"@.len() != "error"@.len());
    }
    let completed = state::can_release_compiler_files("passed", true, true);
    let failed = state::can_release_compiler_files("error", true, true);
    let incomplete = state::can_release_compiler_files("passed", false, true);
    let external = state::can_release_compiler_files("passed", true, false);
    assert(completed && !failed && !incomplete && !external);
}

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
    proof { assert(!state::operation_is_terminal(&rows[2])); }
    assert(!unresolved_last);
    let mut unresolved_first = Vec::new();
    unresolved_first.push(state::OperationObservation { status: "error", process_terminal: None });
    unresolved_first.push(state::OperationObservation { status: "blocked", process_terminal: None });
    let incomplete = state::all_terminal(&unresolved_first);
    proof { assert(!state::operation_is_terminal(&unresolved_first[0])); }
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

}

#[cfg(not(verus_keep_ghost))]
#[test]
fn joined_compiler_retires_only_its_own_outputs_after_writing_the_receipt() {
    use chrono_judge_projects::execute;
    use chrono_judge_registration::execution::{Method, Plan};
    use chrono_judge_routes::prepare;
    use serde_json::json;
    use std::collections::BTreeMap;

    for (output, create_library, passes, retains_files) in [
        ("{operation_state}/compile.rlib", true, true, false),
        ("{operation_state}/compile.rlib", false, false, true),
        ("external.rlib", true, true, true),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let compiler = Method {
            operation: "compile".into(),
            owner: "script:compiler".into(),
            tool: "compiler".into(),
            argv: vec![
                "--crate-type=lib",
                "--no-cheating",
                "--no-external-by-default",
                "--num-threads",
                "2",
                "--output-json",
                "--compile",
                &format!("--emit=link={output},dep-info={output}.d"),
                "fixture.rs",
                "-o",
                output,
                "-Zchecksum-hash-algorithm=sha256",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
            output_contract: Some(chrono_harness::verified_verifier::OutputContract {
                version: "fixture".into(),
                commit: "fixture".into(),
            }),
        };
        let test = "test:compile".to_owned();
        let operation = "compile".to_owned();
        let execution = prepare(
            dir.path(), json!({"operation_state":".chrono-harness/state/compile-run"}),
            &[test.clone()].into_iter().collect(),
            &BTreeMap::from([(test.clone(), Plan {
                operations: vec![operation.clone()], timeout_seconds: 10, output_limit_bytes: 4096,
                operation_bounds: BTreeMap::new(),
            })]),
            &BTreeMap::from([(operation.clone(), vec![compiler])]),
            &BTreeMap::from([(test, operation)]),
            &json!([{"id":"compiler","program":env!("CARGO_BIN_EXE_chrono-projects-test-compiler"),
                "version_argv":["--version"],"expected_version":"chrono-judge-routes-test-tool 0.1.0"}]),
            BTreeMap::new(), &BTreeMap::new(),
        ).unwrap();
        if create_library {
            std::fs::write(dir.path().join("library.fixture"), b"library").unwrap();
        }
        let results = execute(&execution).unwrap();
        let row = &results.executed[0];
        assert_eq!(results.passed(), passes, "{:?}", row.error);
        let receipt = row.receipt.as_ref().unwrap();
        let path = output.replace("{operation_state}", ".chrono-harness/state/compile-run");
        assert_eq!(execution.operations[0].method.argv[10], path);
        assert_eq!(dir.path().join(format!("{path}.d")).exists(), retains_files);
        assert_eq!(
            dir.path().join(&path).exists(),
            retains_files && create_library
        );
        let original = row.original.as_ref().unwrap();
        let raw = std::fs::read(dir.path().join(original["path"].as_str().unwrap())).unwrap();
        assert_eq!(chrono_harness::sha256(&raw), original["sha256"]);
        let stored: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(
            stored["receipt"]["compiler_dependency"],
            json!(receipt.compiler_dependency)
        );
        assert_eq!(
            stored["receipt"]["compiler_artifact"],
            json!(receipt.compiler_artifact)
        );
        assert_eq!(
            chrono_judge_routes::compare(&execution, &execution.operations[0], Some(receipt))
                .is_ok(),
            passes
        );
    }
}
