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
    // Complete operation output is retained once in the receipt. The adjacent
    // process projection intentionally omits large streams.
    let process = &rows[0]["receipt"]["process"];
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
        assert_eq!(actual["schema"], "chrono-cargo-run/v2");
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

#[test]
fn unregistered_ambient_configuration_fails_through_both_runner_profiles() {
    for consumer in [Consumer::Full, Consumer::Scoped] {
        let mut f = Fixture::new();
        write(
            &f.root(),
            ".chrono-harness/state/cargo-home/config.toml",
            "[env]\nCHRONO_AMBIENT='undeclared'\n",
        );
        let (code, report) = f.committed_check("source", consumer);
        assert_ne!(code, 0, "{report}");
        let actual = nested(&report, consumer);
        assert!(
            actual["error"]
                .as_str()
                .unwrap()
                .contains("configuration presence differs"),
            "{actual}"
        );
        assert!(actual["metadata"].is_null() && actual["operation"].is_null());
        assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
    }
}

#[test]
fn documentation_delta_leaves_unrelated_ambient_configuration_unexecuted() {
    for consumer in [Consumer::Full, Consumer::Scoped] {
        let mut f = Fixture::new();
        write(
            &f.root(),
            ".chrono-harness/state/cargo-home/config.toml",
            "[env]\nCHRONO_AMBIENT='undeclared'\n",
        );
        let (code, report) = f.committed_check("docs", consumer);
        assert_eq!(code, 0, "{report}");
        assert!(executions(&report, consumer).is_empty());
        assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
    }
}

#[test]
fn full_and_scoped_consumers_keep_absence_checks_and_documentation_locality() {
    for consumer in [Consumer::Full, Consumer::Scoped] {
        for case in ["stable", "mutation", "docs"] {
            let mut f = Fixture::new();
            let path = absent_input(&mut f);
            if case == "mutation" {
                f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CHRONO_MUTATE"] =
                    json!(path);
                write(
                    &f.root(),
                    "t/src/lib.rs",
                    "#[test] fn create_absent_input() { std::fs::write(std::env::var(\"CHRONO_MUTATE\").unwrap(),b\"created\").unwrap(); }\n",
                );
            }
            let (exit, r) =
                f.committed_check(if case == "docs" { "docs" } else { "source" }, consumer);
            if case == "mutation" {
                assert_ne!(exit, 0, "{r}");
                assert!(
                    r.to_string()
                        .contains("absent input changed during guarded operation"),
                    "{r}"
                );
                assert_eq!(fs::read(path).unwrap(), b"created");
            } else {
                assert_eq!(exit, 0, "{r}");
                if case == "docs" {
                    assert!(executions(&r, consumer).is_empty());
                    assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
                } else {
                    assert_eq!(
                        fs::read(f.root().join(".chrono-harness/state/test-ran")).unwrap(),
                        b"ran"
                    );
                }
            }
        }
    }
}
#[test]
fn compiler_binding_full_and_scoped_consumers_preserve_execution_rejection_and_locality() {
    for consumer in [Consumer::Full, Consumer::Scoped] {
        for case in ["source", "rejected", "docs"] {
            let mut f = Fixture::new();
            compiler_binding(&mut f);
            if case != "source" {
                f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["RUSTC_WRAPPER"] =
                    json!("/bin/false");
                f.save();
            }
            let (exit, report) =
                f.committed_check(if case == "docs" { "docs" } else { "source" }, consumer);
            if case == "docs" {
                assert_eq!(exit, 0, "{report}");
                assert!(executions(&report, consumer).is_empty());
            } else {
                let actual = nested(&report, consumer);
                if case == "source" {
                    assert_eq!(exit, 0, "{report}");
                    assert_eq!(actual["compiler"]["version"]["exit_code"], 0);
                    assert_eq!(actual["operation"]["exit_code"], 0);
                } else {
                    assert_ne!(exit, 0, "{report}");
                    assert!(
                        actual["error"].as_str().unwrap().contains("wrapper"),
                        "{actual}"
                    );
                    assert!(actual["metadata"].is_null() && actual["operation"].is_null());
                }
            }
        }
    }
}
