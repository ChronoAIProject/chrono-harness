use super::*;

fn registration() -> Value {
    json!({"schema":"chrono-shared-startup/v1",
        "directory":".chrono-harness/startup/", "artifact":"fixture-startup",
        "download_action":"actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093",
        "consumers":{
            "unit_example":{"need":"Run the current candidate checker", "output_use":"Install the declared checker before cache preparation and the unchanged check",
                "install":["host-installer","argument with spaces","literal $(not-a-command)"]},
            "aggregate":{"need":"Run the current candidate collector", "output_use":"Install the declared collector before reading unit reports",
                "install":["host-installer","collector"]}}})
}

#[test]
fn declared_startup_is_published_once_and_installed_before_each_actual_consumer() {
    let mut value = source("examples/ci-host-job-gating/.chrono-harness/ci/units.json");
    let base: chrono_ci::units::Config = serde_json::from_value(value.clone()).unwrap();
    let before = chrono_ci::units::render(&base, ".chrono-harness/ci/units.json").unwrap();
    value["job_gating"]["startup"] = registration();
    value["persistent_cache"] = adoption("unit_example");
    let config = serde_json::from_value(value.clone()).unwrap();
    let rendered = chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").unwrap();
    let yaml: Value =
        serde_yaml_ng::from_str(&rendered[".github/workflows/chrono-ci.yml"]).unwrap();
    let old: Value = serde_yaml_ng::from_str(&before[".github/workflows/chrono-ci.yml"]).unwrap();
    assert_eq!(
        yaml["jobs"].as_object().unwrap().len(),
        old["jobs"].as_object().unwrap().len()
    );
    let detector = &yaml["jobs"]["detect"];
    assert_eq!(
        detector["outputs"]["startup_binding"],
        "${{ steps.chrono_startup.outputs.binding }}"
    );
    assert_eq!(
        detector["outputs"]["startup_artifact"],
        "${{ steps.chrono_startup_upload.outputs.artifact-id }}"
    );
    let steps = detector["steps"].as_array().unwrap();
    let produce = steps
        .iter()
        .position(|s| s["id"] == "chrono_startup")
        .unwrap();
    let upload = steps
        .iter()
        .position(|s| s["id"] == "chrono_startup_upload")
        .unwrap();
    let detect = steps.iter().position(|s| s["id"] == "detect").unwrap();
    assert!(produce < upload && upload < detect);
    assert_eq!(steps[upload]["with"]["path"], ".chrono-harness/startup/");
    assert_eq!(steps[upload]["with"]["if-no-files-found"], "error");
    for id in ["unit_example", "aggregate"] {
        let job = &yaml["jobs"][id];
        let steps = job["steps"].as_array().unwrap();
        let download = steps
            .iter()
            .position(|s| s["name"] == "Download registered startup artifact")
            .unwrap();
        let install = steps
            .iter()
            .position(|s| s["name"] == "Install registered startup tools")
            .unwrap();
        let first_use = steps
            .iter()
            .position(|s| {
                s["name"]
                    == if id == "unit_example" {
                        "Prepare registered caches"
                    } else {
                        "Bootstrap registered tools"
                    }
            })
            .unwrap();
        assert!(download < install && install < first_use);
        assert_eq!(
            steps[download]["with"]["artifact-ids"],
            "${{ needs.detect.outputs.startup_artifact }}"
        );
        assert!(
            steps[download]["if"]
                .as_str()
                .unwrap()
                .contains("needs.detect.outputs.startup_artifact != ''")
        );
        assert!(
            steps[install].get("if").is_none(),
            "a missing producer must reach the installer rejection, not skip validation"
        );
        assert!(steps[install].get("continue-on-error").is_none());
        assert_eq!(
            steps[install]["env"]["CHRONO_STARTUP_BINDING"],
            "${{ needs.detect.outputs.startup_binding }}"
        );
        assert!(steps.iter().any(|s| s["name"] == "Canonical harness check"));
    }
    assert!(rendered[".github/workflows/chrono-ci.yml"].contains("'literal $(not-a-command)'"));
    assert!(
        !rendered[".github/workflows/chrono-ci.yml"]
            .contains("artifact-ids: ${{ github.run_attempt")
    );
}

#[test]
fn startup_consumers_and_transfer_paths_must_be_explicit_and_separate_from_evidence() {
    let mut value = source("examples/ci-host-job-gating/.chrono-harness/ci/units.json");
    value["job_gating"]["startup"] = registration();
    for (pointer, replacement) in [
        ("/job_gating/startup/schema", json!("unknown")),
        ("/job_gating/startup/directory", json!("../outside/")),
        (
            "/job_gating/startup/directory",
            json!(".chrono-harness/state/"),
        ),
        (
            "/job_gating/startup/artifact",
            json!("bad ${{ expression }}"),
        ),
        (
            "/job_gating/startup/artifact",
            json!("chrono-detector-evidence"),
        ),
        (
            "/job_gating/startup/download_action",
            json!("actions/download-artifact@v4"),
        ),
        (
            "/job_gating/startup/directory",
            json!(".chrono-harness/startup//"),
        ),
        ("/job_gating/startup/consumers", json!({})),
        ("/job_gating/startup/consumers/unit_example/need", json!("")),
        (
            "/job_gating/startup/consumers/unit_example/install",
            json!([]),
        ),
    ] {
        let mut bad = value.clone();
        *bad.pointer_mut(pointer).unwrap() = replacement;
        let config = serde_json::from_value(bad).unwrap();
        assert!(
            chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").is_err(),
            "{pointer}"
        );
    }
    for id in ["detect", "unit_unknown"] {
        let mut bad = value.clone();
        bad["job_gating"]["startup"]["consumers"][id] =
            registration()["consumers"]["aggregate"].clone();
        let config = serde_json::from_value(bad).unwrap();
        assert!(chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").is_err());
    }
    value["job_gating"]["startup"]["consumers"]
        .as_object_mut()
        .unwrap()
        .remove("aggregate");
    let mut null = value.clone();
    null["job_gating"]["startup"] = Value::Null;
    assert!(serde_json::from_value::<chrono_ci::units::Config>(null).is_err());
    let config = serde_json::from_value(value).unwrap();
    let rendered = chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").unwrap();
    let yaml: Value =
        serde_yaml_ng::from_str(&rendered[".github/workflows/chrono-ci.yml"]).unwrap();
    assert!(
        !yaml["jobs"]["aggregate"]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["name"] == "Install registered startup tools")
    );
}

#[test]
fn shared_producer_preserves_original_cache_save_and_report_identity() {
    let mut value = source("examples/ci-host-job-gating/.chrono-harness/ci/units.json");
    value["job_gating"]["startup"] = registration();
    value["persistent_cache"] = adoption("detect");
    let cache = value["persistent_cache"]["jobs"]["detect"]["caches"].clone();
    value["persistent_cache"]["jobs"]["detect"]["save_after_bootstrap"] = cache;
    let config = serde_json::from_value(value).unwrap();
    let rendered = chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").unwrap();
    let yaml: Value =
        serde_yaml_ng::from_str(&rendered[".github/workflows/chrono-ci.yml"]).unwrap();
    let steps = yaml["jobs"]["detect"]["steps"].as_array().unwrap();
    assert_eq!(
        steps.iter().filter(|s| s["id"] == "chrono_startup").count(),
        1
    );
    assert!(!steps.iter().any(|s| s["id"] == "chrono_cache_bootstrap"));
    for step in steps.iter().filter(|s| {
        s["name"]
            .as_str()
            .unwrap_or("")
            .starts_with("Save registered cache")
    }) {
        assert!(
            step["if"]
                .as_str()
                .unwrap()
                .contains("steps.chrono_startup.outcome == 'success'")
        );
    }
    let report = steps
        .iter()
        .find(|s| s["name"] == "Record original cache transport observations")
        .unwrap();
    assert!(
        report["run"]
            .as_str()
            .unwrap()
            .contains("--bootstrap chrono_startup")
    );
}
