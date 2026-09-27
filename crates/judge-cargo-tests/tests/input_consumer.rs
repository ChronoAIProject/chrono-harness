#[path = "support/cargo.rs"]
mod cargo;
use cargo::*;
use serde_json::{Value, json};
use std::fs;

fn executions(report: &Value, consumer: Consumer) -> &Vec<Value> {
    if consumer == Consumer::Full {
        report["tests"]["executed"].as_array().unwrap()
    } else {
        report["response"]["evidence"]["executed"]
            .as_array()
            .unwrap()
    }
}
fn nested(report: &Value, consumer: Consumer) -> Value {
    let rows = executions(report, consumer);
    assert_eq!(rows.len(), 1, "{report}");
    assert_eq!(rows[0]["operation"], "test.t");
    let process = if consumer == Consumer::Full {
        &rows[0]["receipt"]["process"]
    } else {
        &rows[0]["process"]
    };
    serde_json::from_slice(
        &serde_json::from_value::<Vec<u8>>(process["stdout_bytes"].clone()).unwrap(),
    )
    .unwrap()
}

#[test]
fn full_and_scoped_runners_execute_the_same_registered_cargo_guard() {
    for consumer in [Consumer::Full, Consumer::Scoped] {
        let mut f = Fixture::new();
        let (code, report) = f.committed_check("source", consumer);
        assert_eq!(code, 0, "{report}");
        let actual = nested(&report, consumer);
        assert_eq!(actual["schema"], "chrono-cargo-run/v1");
        assert_eq!(actual["metadata"]["exit_code"], 0);
        assert_eq!(actual["operation"]["exit_code"], 0);
        assert_eq!(
            fs::read(f.root().join(".chrono-harness/state/test-ran")).unwrap(),
            b"ran"
        );
    }
}

#[test]
fn metadata_rejection_blocks_real_test_through_both_runner_profiles() {
    for consumer in [Consumer::Full, Consumer::Scoped] {
        let mut f = Fixture::new();
        let (code, report) = f.committed_check("contract", consumer);
        assert_ne!(code, 0, "{report}");
        let actual = nested(&report, consumer);
        assert_eq!(actual["metadata"]["exit_code"], 0);
        assert!(actual["error"].as_str().unwrap().contains("features"));
        assert!(actual["operation"].is_null());
        assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
    }
}

#[test]
fn documentation_delta_does_not_run_an_unrelated_historical_cargo_defect() {
    for consumer in [Consumer::Full, Consumer::Scoped] {
        let mut f = Fixture::new();
        f.contract["packages"][2]["features"] = json!([]);
        f.save();
        let (code, report) = f.committed_check("docs", consumer);
        assert_eq!(code, 0, "{report}");
        assert!(executions(&report, consumer).is_empty());
        assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
    }
}
