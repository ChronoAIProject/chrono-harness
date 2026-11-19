#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
verus! {
fn failed_inventory_keeps_its_failed_outcome() {
    let status = super::policy::inventory_output::listing_status(false);
    assert(status@ == "failed"@);
}
}

#[cfg(not(verus_keep_ghost))]
use chrono_harness::sha256;
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::{fs, path::Path};

#[cfg(not(verus_keep_ghost))]
#[allow(dead_code)]
#[path = "support/tools.rs"]
mod tools;

#[cfg(not(verus_keep_ghost))]
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

#[cfg(not(verus_keep_ghost))]
fn execute(root: &Path) -> Result<String, String> {
    chrono_ci::dispatch(&[
        "test-inventory".into(),
        "--host-root".into(),
        root.to_str().unwrap().into(),
        "--config".into(),
        "config.json".into(),
    ])
}

#[cfg(not(verus_keep_ghost))]
fn read(root: &Path, path: &str) -> Value {
    serde_json::from_slice(&fs::read(root.join(path)).unwrap()).unwrap()
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn adopted_inventory_keeps_each_completed_or_failed_listing_in_its_own_window() {
    if !tools::isolated_git() {
        return;
    }
    let host = tools::temporary_host("retained inventory ");
    let root = host.path();
    assert!(
        std::process::Command::new(tools::fixture_git())
            .args(["init", "-q"])
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
    fs::create_dir_all(root.join(".chrono-harness/state")).unwrap();
    let policy = json!({
        "schema":"chrono-output-retention/v2","producers":{"check-original":[".chrono-harness/state/"]},
        "keep_seconds":0,"diagnostic_keep_seconds":86400,"max_lifetime_seconds":86400,
        "max_related_outputs":512,"max_summaries":128,"max_nodes_per_round":256,
        "max_bytes_per_round":67108864,"max_millis_per_round":25
    });
    fs::write(
        root.join(".chrono-harness/retention.json"),
        policy.to_string(),
    )
    .unwrap();
    let configure = |mode| {
        fixture(root, mode);
        let mut config = read(root, "config.json");
        config["report_directory"] = json!(".chrono-harness/state/inventory");
        fs::write(root.join("config.json"), config.to_string()).unwrap();
    };
    configure("ok");
    let result: Value = serde_json::from_str(&execute(root).unwrap()).unwrap();
    let directory = result["report_directory"].as_str().unwrap();
    assert!(directory.starts_with(".chrono-harness/state/inventory-"));
    let report = read(root, &format!("{directory}/inventory.json"));
    let original = report["receipts"][0]["stdout"].as_str().unwrap();
    let bytes = fs::read(root.join(original)).unwrap();
    configure("failed");
    let error = execute(root).unwrap_err();
    let failed = error.split_once("report_directory=").unwrap().1;
    assert_ne!(directory, failed);
    let receipts = read(root, &format!("{failed}/receipts.json"));
    let failed_listing = receipts.as_array().unwrap().last().unwrap();
    assert_eq!(failed_listing["action"], "right");
    assert_eq!(failed_listing["exit_code"], 7);
    let stderr = fs::read(root.join(failed_listing["stderr"].as_str().unwrap())).unwrap();
    assert_eq!(stderr, b"original failure\n");
    assert_eq!(failed_listing["stderr_sha256"], sha256(&stderr));
    assert_eq!(fs::read(root.join(original)).unwrap(), bytes);
    assert!(!root.join(failed).join("inventory.json").exists());
    let owner = chrono_harness::retained_artifacts::retention::Inventory::adopted(root)
        .unwrap()
        .unwrap();
    for (path, expected_status) in [(directory, "passed"), (failed, "failed")] {
        let observed = owner.producer_status("check-original", Some(path)).unwrap();
        let record = &observed["records"][0];
        assert_eq!(record["outcome"]["status"], expected_status);
        assert_eq!(record["sealed"], true);
        let born = record["lifetime"]["created_at"].as_u64().unwrap();
        let deadline = record["lifetime"]["deadline"].as_u64().unwrap();
        assert_eq!(deadline - born, 86400);
        assert_eq!(record["expires"], deadline);
        let original = [record["id"].as_str().unwrap().to_owned()];
        owner
            .maintain_originals_at(&original, deadline - 1)
            .unwrap();
        assert!(root.join(path).is_dir());
        for _ in 0..32 {
            let round = owner.maintain_originals_at(&original, deadline).unwrap();
            assert_eq!(round["accounting_errors"], json!([]));
            if !root.join(path).exists() {
                break;
            }
        }
        assert!(!root.join(path).exists());
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn explicit_test_inventory_keeps_complete_disjoint_groups_and_original_receipts() {
    if !tools::isolated_git() {
        return;
    }
    let host = tools::temporary_host("inventory λ ");
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

#[cfg(not(verus_keep_ghost))]
#[test]
fn test_inventory_rejects_missing_overlapping_ignored_drifting_and_malformed_lists() {
    if !tools::isolated_git() {
        return;
    }
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
        let host = tools::temporary_host("inventory invalid ");
        fixture(host.path(), mode);
        let error = execute(host.path()).unwrap_err();
        assert!(error.contains(expected), "{mode}: {error}");
        assert!(!host.path().join("reports/inventory.json").exists());
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn test_inventory_keeps_nonzero_timeout_and_output_bound_originals() {
    if !tools::isolated_git() {
        return;
    }
    for (mode, expected) in [
        ("failed", "failed listing"),
        ("timeout", "timed out"),
        ("overflow", "output limit"),
    ] {
        let host = tools::temporary_host("inventory process ");
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
