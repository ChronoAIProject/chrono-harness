use chrono_harness::sha256;
use serde_json::{Value, json};
use std::{fs, path::Path};

fn fixture(root: &Path, mode: &str) {
    let groups = json!({"t":"left","opaque":"right"});
    let mut actions = json!({});
    for action in ["all", "left", "right"] {
        actions[action] = json!({"operation":action,"tool":"listing","argv":[
            "inventory-listing",mode,action
        ]});
    }
    fs::write(
        root.join("projects.json"),
        json!({"projects":[{
            "id":"t","test_groups":groups,"actions":actions
        }]})
        .to_string(),
    )
    .unwrap();
    let mut config = json!({
        "schema":"chrono-workflow-inventory/v1","projects":"projects.json","project":"t",
        "groups":groups,"unfiltered_action":"all",
        "tools":{"listing":env!("CARGO_BIN_EXE_chrono-ci-test-transport")},
        "binaries":{"all":["cases"],"left":["cases"],"right":["cases"]},
        "report_directory":"reports","timeout_seconds":5,"output_limit_bytes":4096
    });
    if mode == "drift" {
        config["groups"] = json!({"t":"right","opaque":"left"});
    }
    if mode == "timeout" {
        config["timeout_seconds"] = json!(0.1);
    }
    fs::write(root.join("config.json"), config.to_string()).unwrap();
}

fn execute(root: &Path) -> Result<String, String> {
    chrono_ci::dispatch(&[
        "test-inventory".into(),
        "--host-root".into(),
        root.to_str().unwrap().into(),
        "--config".into(),
        "config.json".into(),
    ])
}

fn read(root: &Path, path: &str) -> Value {
    serde_json::from_slice(&fs::read(root.join(path)).unwrap()).unwrap()
}

#[test]
fn explicit_test_inventory_keeps_complete_disjoint_groups_and_original_receipts() {
    let host = tempfile::Builder::new()
        .prefix("inventory λ ")
        .tempdir()
        .unwrap();
    let root = host.path();
    fixture(root, "ok");
    let summary: Value = serde_json::from_str(&execute(root).unwrap()).unwrap();
    assert_eq!(summary, json!({"complete":2,"groups":{"t":1,"opaque":1}}));
    let report = read(root, "reports/inventory.json");
    assert_eq!(report["complete"], json!(["cases::a", "cases::b"]));
    assert_eq!(
        report["groups"],
        json!({"t":["cases::a"],"opaque":["cases::b"]})
    );
    for (field, path) in [
        ("config_sha256", "config.json"),
        ("projects_sha256", "projects.json"),
    ] {
        assert_eq!(report[field], sha256(&fs::read(root.join(path)).unwrap()));
    }
    let receipts = report["receipts"].as_array().unwrap();
    assert_eq!(receipts.len(), 4);
    for receipt in receipts {
        assert_eq!(receipt["exit_code"], 0);
        assert!(receipt["error"].is_null());
        assert_eq!(
            receipt["argv"].as_array().unwrap().last().unwrap(),
            "--list"
        );
        for stream in ["stdout", "stderr"] {
            let raw = fs::read(root.join(receipt[stream].as_str().unwrap())).unwrap();
            assert_eq!(receipt[format!("{stream}_sha256")], sha256(&raw));
        }
    }
}

#[test]
fn test_inventory_rejects_missing_overlapping_ignored_drifting_and_malformed_lists() {
    for (mode, expected) in [
        ("empty", "empty"),
        ("overlap", "overlapping"),
        ("omit", "omission"),
        ("ignored", "ignored"),
        ("drift", "binding drift"),
        ("duplicate", "duplicate"),
        ("truncated", "incomplete"),
        ("count", "malformed"),
        ("binary", "binaries differ"),
        ("bench", "malformed"),
        ("unexpected", "unexpected"),
    ] {
        let host = tempfile::tempdir().unwrap();
        fixture(host.path(), mode);
        let error = execute(host.path()).unwrap_err();
        assert!(error.contains(expected), "{mode}: {error}");
        assert!(!host.path().join("reports/inventory.json").exists());
    }
}

#[test]
fn test_inventory_keeps_nonzero_timeout_and_output_bound_originals() {
    for (mode, expected) in [
        ("failed", "failed listing"),
        ("timeout", "timed out"),
        ("overflow", "output limit"),
    ] {
        let host = tempfile::tempdir().unwrap();
        let root = host.path();
        fixture(root, mode);
        let error = execute(root).unwrap_err();
        assert!(error.contains(expected), "{mode}: {error}");
        let receipts = read(root, "reports/receipts.json");
        let last = receipts.as_array().unwrap().last().unwrap();
        if mode == "failed" {
            assert_eq!(last["exit_code"], 7);
            assert_eq!(
                fs::read(root.join(last["stderr"].as_str().unwrap())).unwrap(),
                b"original failure\n"
            );
        } else {
            assert!(last["error"].as_str().unwrap().contains(expected));
        }
        for stream in ["stdout", "stderr"] {
            let raw = fs::read(root.join(last[stream].as_str().unwrap())).unwrap();
            assert!(raw.len() <= 4096);
            assert_eq!(last[format!("{stream}_sha256")], sha256(&raw));
        }
    }
}
