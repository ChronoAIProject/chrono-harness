#[path = "support/execution.rs"]
mod fixture;
use chrono_host_release::{
    receipt::{self, Kind, Lineage},
    recipe::Loaded,
};
use serde_json::{Value, json};
use std::fs;

#[test]
fn actual_non_rust_unit_runs_original_literal_action_without_resolving_sdk() {
    let mut f = fixture::Fixture::new("pass", false);
    for name in ["cargo", "rustc", "rustup"] {
        f.host.config["tools"][name] = json!("unavailable-release-sdk");
    }
    f.host.save();
    f.git(&["add", "--all"]);
    f.git(&[
        "-c",
        "user.name=Release test",
        "-c",
        "user.email=release-test@example.invalid",
        "commit",
        "--quiet",
        "-m",
        "no SDK bindings",
    ]);
    f.success("test_none");
    let report = f.report("test_none");
    assert_eq!(report["status"], "passed");
    assert!(report.get("toolchain").is_none());
    assert_eq!(report["execution_tools"].as_object().unwrap().len(), 2);
    let process = report["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["phase"] == "verification")
        .unwrap();
    assert_eq!(
        process["argv"]
            .as_array()
            .unwrap()
            .iter()
            .skip(1)
            .cloned()
            .collect::<Vec<_>>(),
        vec![
            json!("verify none"),
            json!("space value"),
            json!("$literal"),
            json!("λ")
        ]
    );
    let loaded = Loaded::load(f.root()).unwrap();
    let parent = Lineage {
        kind: Kind::Local,
        run_id: "self-contained-run".into(),
        attempt: "1".into(),
        job: "collect".into(),
    };
    let selection=serde_json::from_value(json!({"job":"test_none","artifact_id":f.outcomes["test_none"]["outputs"]["artifact_id"],"run_id":"self-contained-run","attempt":"1"})).unwrap();
    let verified = receipt::read(
        &loaded,
        f.root(),
        &report["context"],
        &parent,
        loaded.unit("test_none").unwrap(),
        &selection,
    )
    .unwrap();
    receipt::verify_consumption(&loaded, &verified, &json!({})).unwrap();
}

#[test]
fn actual_build_verify_collect_invokes_real_distribution_pack_and_retains_originals() {
    let mut f = fixture::Fixture::new("pass", false);
    for id in ["build_a", "build_b", "test_none", "test_a"] {
        f.success(id);
    }
    let metadata = json!(f.outcomes);
    fs::write(
        f.root().join("dependencies.json"),
        serde_json::to_vec(&metadata).unwrap(),
    )
    .unwrap();
    let output = f.outputs.path().join("collection");
    let out = f.invoke_at("collect", &output, &metadata, true);
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let report: Value =
        serde_json::from_slice(&fs::read(output.join("receipt.json")).unwrap()).unwrap();
    assert_eq!(report["status"], "passed");
    assert!(report.get("toolchain").is_none());
    assert_eq!(report["originals"].as_object().unwrap().len(), 4);
    assert_eq!(report["verifications"].as_object().unwrap().len(), 2);
    let release: Value =
        serde_json::from_slice(&fs::read(output.join("release.json")).unwrap()).unwrap();
    let platform = chrono_distribution::platform();
    assert_eq!(release["source_commit"], report["context"]["source_commit"]);
    assert_eq!(
        release["platforms"][&platform].as_object().unwrap().len(),
        2
    );
    for (asset, declaration) in report["package"]["assets"].as_object().unwrap() {
        let original = fs::read(
            f.root()
                .join(f.host.plan["assets"][asset].as_str().unwrap()),
        )
        .unwrap();
        let packaged = fs::read(output.join(declaration["path"].as_str().unwrap())).unwrap();
        assert_eq!(original, packaged);
    }
    for (id, files) in report["originals"].as_object().unwrap() {
        for (name, declaration) in files.as_object().unwrap() {
            let raw = fs::read(output.join("originals").join(id).join(name)).unwrap();
            assert_eq!(chrono_harness::sha256(&raw), declaration["sha256"]);
            assert_eq!(raw.len() as u64, declaration["size"].as_u64().unwrap());
        }
    }
}

#[test]
fn current_failure_rejects_old_passing_build_before_running_the_verifier() {
    let mut f = fixture::Fixture::new("pass", false);
    f.success("build_a");
    let original = fs::read(f.handoff("build_a").join("receipt.json")).unwrap();
    let mut dependency = f.outcomes["build_a"].clone();
    dependency["result"] = json!("failure");
    let out = f.invoke_at(
        "test_a",
        &f.handoff("test_a"),
        &json!({"build_a":dependency}),
        false,
    );
    assert_eq!(out.status.code(), Some(2));
    let report = f.report("test_a");
    assert_eq!(report["status"], "failed");
    assert!(
        !report["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["phase"] == "verification")
    );
    assert_eq!(
        fs::read(f.handoff("build_a").join("receipt.json")).unwrap(),
        original
    );
}

#[test]
fn actual_failed_child_keeps_binary_streams_nested_and_registered_evidence() {
    let mut f = fixture::Fixture::new("fail", false);
    let out = f.invoke("test_none");
    assert_eq!(out.status.code(), Some(37));
    let report = f.report("test_none");
    assert_eq!(report["failure"]["exit_code"], 37);
    assert_eq!(report["failure"]["phase"], "verification");
    let process = report["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["phase"] == "verification")
        .unwrap();
    assert_eq!(
        fs::read(
            f.handoff("test_none")
                .join(process["stdout"]["path"].as_str().unwrap())
        )
        .unwrap(),
        [0u8, 255, 10]
    );
    assert_eq!(process["failure_evidence"]["status"], "retained");
    assert_eq!(process["registered_failure_evidence"]["status"], "retained");
    let nested = process["failure_evidence"]["bytes"].as_u64().unwrap();
    assert_eq!(
        process["registered_failure_evidence"]["bound_bytes"]
            .as_u64()
            .unwrap(),
        64 * 1024 * 1024 - nested
    );
    let retained = &process["registered_failure_evidence"]["files"][0];
    assert_eq!(
        fs::read(
            f.handoff("test_none")
                .join(retained["path"].as_str().unwrap())
        )
        .unwrap(),
        [0u8, 254, 255]
    );
}

#[test]
fn post_verification_drift_fails_success_and_preserves_the_original_child_failure() {
    for (behavior, expected, child) in [("drift", 2, 0), ("fail-and-drift", 37, 37)] {
        let mut f = fixture::Fixture::new(behavior, false);
        f.success("build_a");
        let out = f.invoke("test_a");
        assert_eq!(out.status.code(), Some(expected));
        let report = f.report("test_a");
        assert_eq!(report["status"], "failed");
        assert_eq!(
            report["consumer_staging"]["observations"][2]["matches"],
            false
        );
        let verification = report["processes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["phase"] == "verification")
            .unwrap();
        assert_eq!(verification["exit_code"], child);
        if child != 0 {
            assert_eq!(report["failure"]["exit_code"], child);
        }
    }
}

#[test]
fn tracked_consumer_is_rejected_before_it_is_replaced_or_executed() {
    let mut f = fixture::Fixture::new("pass", true);
    f.success("build_a");
    let out = f.invoke("test_a");
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(
        fs::read(f.root().join("consumer/a")).unwrap(),
        b"tracked original"
    );
    let report = f.report("test_a");
    assert!(
        !report["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["phase"] == "verification")
    );
}

#[test]
fn malformed_recipe_or_existing_output_fails_before_any_unit_process() {
    let f = fixture::Fixture::new("pass", false);
    let output = f.outputs.path().join("existing");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("protected"), b"existing").unwrap();
    let out = f.invoke_at("test_none", &output, &json!({}), false);
    assert_eq!(out.status.code(), Some(2));
    assert_eq!(fs::read(output.join("protected")).unwrap(), b"existing");
    assert!(!output.join("receipt.json").exists());
    let mut config = f.host.config.clone();
    config["schema"] = json!("old-release-schema");
    fs::write(
        f.root().join(".chrono-harness/release/build.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let absent = f.outputs.path().join("absent");
    let out = f.invoke_at("test_none", &absent, &json!({}), false);
    assert_eq!(out.status.code(), Some(2));
    assert!(!absent.exists());
}

#[test]
fn wrong_compiler_is_retained_as_failed_original_observation_without_building() {
    let mut f = fixture::Fixture::new("wrong-compiler", false);
    let out = f.invoke("build_a");
    assert_eq!(out.status.code(), Some(2));
    let report = f.report("build_a");
    assert_eq!(report["status"], "failed");
    assert_eq!(
        report["processes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["phase"] == "compiler-version")
            .count(),
        2
    );
    assert!(
        !report["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["phase"] == "build")
    );
    assert!(!f.handoff("build_a").join("executable.tar").exists());
}

#[test]
fn nested_failure_evidence_rejects_ancestor_symlink_before_directory_enumeration() {
    let root = tempfile::tempdir().unwrap();
    let output = tempfile::tempdir().unwrap();
    let nested = tempfile::Builder::new()
        .prefix("chrono-native-publication-failure-")
        .tempdir()
        .unwrap();
    for name in ["stdout", "stderr", "process.json"] {
        fs::write(nested.path().join(name), b"original").unwrap();
    }
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir_all(outside.path().join("collection/preparation")).unwrap();
    fs::write(
        outside
            .path()
            .join("collection/preparation/should-not-enumerate"),
        b"external",
    )
    .unwrap();
    std::os::unix::fs::symlink(outside.path(), nested.path().join("state")).unwrap();
    let marker = format!(
        "original native publication process evidence: {}\n",
        nested.path().display()
    );
    let retained = chrono_host_release::evidence::retain(
        root.path(),
        output.path(),
        "verification",
        &[],
        &[],
        marker.as_bytes(),
    );
    let record = retained.nested.unwrap();
    assert_eq!(record["status"], "partial");
    assert_eq!(record["files"].as_array().unwrap().len(), 4);
    assert_eq!(record["files"][3]["source"], "state/collection/preparation");
    assert_ne!(record["files"][3]["status"], "retained");
    assert!(!record.to_string().contains("should-not-enumerate"));
    assert!(
        !output
            .path()
            .join("failure-evidence/verification/state")
            .exists()
    );
}

#[test]
fn local_driver_publishes_real_release_from_isolated_roots_with_registered_worker_cap() {
    for workers in [1usize, 2] {
        let mut f = fixture::Fixture::new("pass", false);
        f.host.config["local_workers"] = json!(workers);
        f.host.save();
        if workers != 2 {
            f.git(&["add", "--all"]);
            f.git(&[
                "-c",
                "user.name=Release test",
                "-c",
                "user.email=release-test@example.invalid",
                "commit",
                "--quiet",
                "-m",
                "local worker selection",
            ]);
        }
        let output = f.outputs.path().join("local");
        let out = f.local(&output);
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let local: Value =
            serde_json::from_slice(&fs::read(output.join("local.json")).unwrap()).unwrap();
        assert_eq!(local["status"], "passed");
        assert_eq!(local["units"].as_object().unwrap().len(), 5);
        let receipt: Value =
            serde_json::from_slice(&fs::read(output.join("receipt.json")).unwrap()).unwrap();
        assert_eq!(receipt["status"], "passed");
        assert!(output.join("release.json").is_file());
        assert!(!output.join("units/collect").exists());
        let mut events = Vec::new();
        for id in ["build_a", "build_b", "test_none", "test_a"] {
            let unit = &local["units"][id];
            assert_eq!(unit["result"], "success");
            assert!(!output.join("roots").join(id).exists());
            let processes = unit["processes"].as_array().unwrap();
            let source = processes[1]["cwd"].as_str().unwrap();
            assert!(source.ends_with(&format!("/roots/{id}")));
            events.push((processes[0]["started_ns"].as_u64().unwrap(), 1i32));
            events.push((
                processes.last().unwrap()["finished_ns"].as_u64().unwrap(),
                -1,
            ));
            for process in processes {
                for stream in ["stdout", "stderr"] {
                    let declaration = &process[stream];
                    let raw = fs::read(output.join(declaration["path"].as_str().unwrap())).unwrap();
                    assert_eq!(chrono_harness::sha256(&raw), declaration["sha256"]);
                    assert_eq!(raw.len(), declaration["size"].as_u64().unwrap() as usize);
                }
            }
        }
        events.sort();
        let (mut active, mut peak) = (0, 0);
        for (_, change) in events {
            active += change;
            peak = peak.max(active);
        }
        assert_eq!(active, 0);
        assert!(
            peak > 0 && peak <= workers as i32,
            "workers {workers}, observed {peak}"
        );
    }
}

#[test]
fn local_driver_keeps_original_failure_and_finishes_independent_siblings() {
    let f = fixture::Fixture::new("fail", false);
    let output = f.outputs.path().join("local");
    let out = f.local(&output);
    assert_eq!(out.status.code(), Some(2));
    let local: Value =
        serde_json::from_slice(&fs::read(output.join("local.json")).unwrap()).unwrap();
    assert_eq!(local["status"], "failed");
    assert_eq!(local["units"].as_object().unwrap().len(), 5);
    for id in ["build_a", "build_b"] {
        assert_eq!(local["units"][id]["result"], "success");
    }
    for id in ["test_none", "test_a", "collect"] {
        assert_eq!(local["units"][id]["result"], "failure");
    }
    for id in ["test_none", "test_a"] {
        let original: Value = serde_json::from_slice(
            &fs::read(output.join("units").join(id).join("receipt.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(original["failure"]["exit_code"], 37);
        assert!(
            local["units"][id]["retained_root"]
                .as_str()
                .is_some_and(|path| std::path::Path::new(path).is_dir())
        );
    }
    assert!(!output.join("release.json").exists());
}

#[test]
fn local_driver_rejects_dirty_source_and_existing_output_before_unit_effects() {
    let f = fixture::Fixture::new("pass", false);
    let existing = f.outputs.path().join("existing");
    fs::create_dir(&existing).unwrap();
    fs::write(existing.join("protected"), b"existing").unwrap();
    assert_eq!(f.local(&existing).status.code(), Some(2));
    assert_eq!(fs::read(existing.join("protected")).unwrap(), b"existing");
    fs::write(f.root().join("dirty-source"), b"uncommitted").unwrap();
    let absent = f.outputs.path().join("absent");
    assert_eq!(f.local(&absent).status.code(), Some(2));
    assert!(!absent.exists());
}
