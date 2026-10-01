use chrono_harness::parity;
use serde_json::{Value, json};

fn report(complete: bool) -> Value {
    json!({
        "schema_version": 1,
        "scope": "configured-judges",
        "status": "pass",
        "base": "a".repeat(64),
        "candidate": "b".repeat(64),
        "candidate_tree": "c".repeat(64),
        "context_digest": "d".repeat(64),
        "registry_digest": "e".repeat(64),
        "environment": {"inherited": {"LANG": "C"}, "effective": {"LANG": "C"}},
        "executables": [{"path":"/local/chrono-harness","sha256":"f".repeat(64),"version":"0.1.0"}],
        "tools": [{"id":"git","sha256":"1".repeat(64),"version":"2.46"}],
        "effective_inputs": {"completeness_proven": complete, "identity":"2".repeat(64)},
        "delta": [],
        "impact": {"seeds":[],"tests":[]},
        "judges": [{
            "id":"registration",
            "state":"executed",
            "exit_code":0,
            "binding":{"id":"registration","executable":"judge","version":"0.1.0"},
            "response":{
                "protocol":"chrono-judge/v1",
                "judge_id":"registration",
                "status":"pass",
                "findings":[],
                "evidence":{},
                "outputs":{"impact":{"seeds":[]}}
            },
            "process":{"exit_code":0,"failure":null,"stdout_sha256":"3".repeat(64),"stderr_sha256":"4".repeat(64),"sha256":"5".repeat(64)}
        }],
        "tests": {"selected":[],"executed":[]},
        "findings": [],
        "unresolved": {},
        "costs": null
    })
}

#[test]
fn matching_contributions_cannot_be_completed_parity_evidence() {
    for marker in ["request-scope", "results-scope"] {
        let mut unit = report(true);
        if marker == "request-scope" {
            unit["execution_scope"] = json!({"kind":"unit","unit":"one"});
        } else {
            unit["tests"]["scope"] = json!("unit:one");
        }
        let error = parity::establish(&unit, &unit, "other.json").unwrap_err();
        assert!(error.contains("contribution-only"), "{error}");
    }
}

#[test]
fn equal_complete_reports_establish_pairwise_parity() {
    let mut other = report(true);
    other["executables"][0]["path"] = "/ci/chrono-harness".into();
    let parity = parity::establish(&report(true), &other, ".chrono-harness/state/ci.json").unwrap();
    assert_eq!(parity["status"], "established");
    assert_eq!(parity["compared_report"], ".chrono-harness/state/ci.json");
    assert_eq!(parity["evidence"]["kind"], "pairwise-report-v1");
}

#[test]
fn incomplete_inputs_fail_closed_even_when_reports_match() {
    let error = parity::establish(&report(false), &report(false), "other.json").unwrap_err();
    assert!(error.contains("complete effective inputs"), "{error}");
}

#[test]
fn verdict_difference_is_reported_as_unestablished() {
    let mut other = report(true);
    other["tests"]["executed"] = json!(["test:new"]);
    let error = parity::establish(&report(true), &other, "other.json").unwrap_err();
    assert!(error.contains("verdict-bearing reports differ"), "{error}");
}

#[test]
fn update_files_publishes_parity_to_canonical_and_retained_reports() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".chrono-harness/state")).unwrap();
    let mut current = report(true);
    current["report_path"] = ".chrono-harness/state/run-local.json".into();
    let mut compared = report(true);
    compared["executables"][0]["path"] = "/ci/chrono-harness".into();
    std::fs::write(
        dir.path().join(".chrono-harness/state/report.json"),
        serde_json::to_vec_pretty(&current).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join(".chrono-harness/state/ci.json"),
        serde_json::to_vec_pretty(&compared).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join(".chrono-harness/state/run-local.json"),
        serde_json::to_vec_pretty(&current).unwrap(),
    )
    .unwrap();
    let (updated, established) = chrono_harness::parity::update_files(
        dir.path(),
        std::path::Path::new(".chrono-harness/state/report.json"),
        std::path::Path::new(".chrono-harness/state/ci.json"),
    )
    .unwrap();
    assert!(established);
    assert_eq!(updated["parity"]["status"], "established");
    let retained: Value = serde_json::from_slice(
        &std::fs::read(dir.path().join(".chrono-harness/state/run-local.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(retained["parity"]["status"], "established");
}

#[test]
fn update_files_preserves_unresolved_reason_and_nonzero_state() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".chrono-harness/state")).unwrap();
    let current = report(false);
    std::fs::write(
        dir.path().join(".chrono-harness/state/report.json"),
        serde_json::to_vec_pretty(&current).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join(".chrono-harness/state/ci.json"),
        serde_json::to_vec_pretty(&report(false)).unwrap(),
    )
    .unwrap();
    let (updated, established) = chrono_harness::parity::update_files(
        dir.path(),
        std::path::Path::new(".chrono-harness/state/report.json"),
        std::path::Path::new(".chrono-harness/state/ci.json"),
    )
    .unwrap();
    assert!(!established);
    assert!(
        updated["unresolved"]["/parity"]
            .as_str()
            .unwrap()
            .contains("complete effective inputs")
    );
}

#[test]
fn parity_cli_publishes_established_evidence_without_rerunning_judges() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".chrono-harness/state")).unwrap();
    let mut current = report(true);
    current["report_path"] = ".chrono-harness/state/run.json".into();
    let mut compared = report(true);
    compared["executables"][0]["path"] = "/ci/chrono-harness".into();
    for (name, value) in [
        ("report.json", &current),
        ("run.json", &current),
        ("ci.json", &compared),
    ] {
        std::fs::write(
            dir.path().join(".chrono-harness/state").join(name),
            serde_json::to_vec_pretty(value).unwrap(),
        )
        .unwrap();
    }
    let old = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    let output = chrono_harness::dispatch(&[
        "parity",
        "--host-root",
        ".",
        "--report",
        ".chrono-harness/state/report.json",
        "--compared-report",
        ".chrono-harness/state/ci.json",
    ]);
    std::env::set_current_dir(old).unwrap();
    assert_eq!(output.exit_code, 0, "{}", output.stderr);
    let printed: Value = serde_json::from_str(&output.stdout).unwrap();
    assert_eq!(printed["parity"]["status"], "established");
}
