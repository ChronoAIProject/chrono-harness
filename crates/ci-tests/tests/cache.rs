use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn source(path: &str) -> Value {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    serde_json::from_slice(&fs::read(root.join(path)).unwrap()).unwrap()
}

fn adoption(job: &str) -> Value {
    json!({"schema":"chrono-github-cache/v1",
        "program":".chrono-harness/bin/chrono-cache", "config":".chrono-harness/cache.json",
        "restore_action":"actions/cache/restore@0057852bfaa89a56745cba8c7296529d2fc39830",
        "save_action":"actions/cache/save@0057852bfaa89a56745cba8c7296529d2fc39830",
        "jobs":{job:{"need":"Reuse this existing job's compilation outputs between runners", "output_use":"The original registered build revalidates the restored target before checking", "prepare":["/usr/bin/printf","%s","literal ' argument $(printf unsafe)"], "consumer":"host.build", "caches":["core.target"]}}})
}

#[test]
fn explicit_save_ownership_keeps_restores_and_original_checks_in_read_only_consumers() {
    let mut value = source("examples/ci-host-job-gating/.chrono-harness/ci/units.json");
    value["persistent_cache"] = adoption("unit_example");
    value["persistent_cache"]["jobs"]["unit_example"]["save_caches"] = json!([]);
    let config = serde_json::from_value(value.clone()).unwrap();
    let rendered = chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").unwrap();
    let yaml: Value =
        serde_yaml_ng::from_str(&rendered[".github/workflows/chrono-ci.yml"]).unwrap();
    let steps = yaml["jobs"]["unit_example"]["steps"].as_array().unwrap();
    assert!(steps.iter().any(|s| s["id"] == "cache_0_restore"));
    assert!(!steps.iter().any(|s| s["id"] == "cache_0_save"));
    assert!(steps.iter().any(|s| {
        s["run"]
            .as_str()
            .is_some_and(|s| s.contains("'check' '--unit' 'example'"))
    }));
    assert!(steps.iter().any(|s| {
        s["run"]
            .as_str()
            .is_some_and(|s| s.contains("--save-caches '[]'"))
    }));
    for saves in [json!(["unknown"]), json!(["core.target", "core.target"])] {
        value["persistent_cache"]["jobs"]["unit_example"]["save_caches"] = saves;
        let config = serde_json::from_value(value.clone()).unwrap();
        assert!(chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").is_err());
    }
    value["persistent_cache"]["jobs"]["unit_example"]["save_caches"] = json!([]);
    value["persistent_cache"]["jobs"]["unit_example"]["save_after_bootstrap"] =
        json!(["core.target"]);
    let config = serde_json::from_value(value).unwrap();
    assert!(chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").is_err());
}

#[test]
fn detector_retains_declared_originals_after_cache_reporting_even_when_work_fails() {
    let mut value = source("examples/ci-host-job-gating/.chrono-harness/ci/units.json");
    value["persistent_cache"] = adoption("detect");
    value["collection"]["include_hidden_files"] = json!(true);
    let baseline = chrono_ci::units::render(
        &serde_json::from_value(value.clone()).unwrap(),
        ".chrono-harness/ci/units.json",
    )
    .unwrap();
    value["job_gating"]["detector"]["evidence_directory"] = json!(".chrono-harness/state/");
    let config = serde_json::from_value(value.clone()).unwrap();
    let rendered = chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").unwrap();
    let yaml: Value =
        serde_yaml_ng::from_str(&rendered[".github/workflows/chrono-ci.yml"]).unwrap();
    let old: Value = serde_yaml_ng::from_str(&baseline[".github/workflows/chrono-ci.yml"]).unwrap();
    assert_eq!(
        yaml["jobs"].as_object().unwrap().len(),
        old["jobs"].as_object().unwrap().len()
    );
    assert_eq!(yaml["jobs"]["unit_example"], old["jobs"]["unit_example"]);
    assert_eq!(yaml["jobs"]["aggregate"], old["jobs"]["aggregate"]);
    let steps = yaml["jobs"]["detect"]["steps"].as_array().unwrap();
    let upload = steps.last().unwrap();
    assert_eq!(upload["if"], "${{ always() }}");
    assert_eq!(
        upload["uses"],
        value["collection"]["upload_artifact_action"]
    );
    assert_eq!(upload["with"]["path"], ".chrono-harness/state/");
    assert_eq!(upload["with"]["if-no-files-found"], "error");
    assert_eq!(upload["with"]["include-hidden-files"], true);
    assert_eq!(
        steps[steps.len() - 2]["name"],
        "Record original cache transport observations"
    );
    for directory in [
        "../outside/",
        ".chrono-harness/",
        ".chrono-harness/state",
        ".chrono-harness/state/*/",
        ".chrono-harness/state/${{ github.token }}/",
    ] {
        value["job_gating"]["detector"]["evidence_directory"] = json!(directory);
        let config = serde_json::from_value(value.clone()).unwrap();
        assert!(
            chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").is_err(),
            "{directory}"
        );
    }
    value["job_gating"]["detector"]["evidence_directory"] = json!(".chrono-harness/state/");
    value["collection"]["include_hidden_files"] = json!(false);
    let config = serde_json::from_value(value).unwrap();
    assert!(chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").is_err());
}

#[test]
fn gated_jobs_restore_before_bootstrap_and_save_without_replacing_the_check() {
    let mut value = source("examples/ci-host-job-gating/.chrono-harness/ci/units.json");
    value["persistent_cache"] = adoption("unit_example");
    value["persistent_cache"]["jobs"]["unit_example"]["caches"] =
        json!(["core.target", "a.target"]);
    let config = serde_json::from_value(value).unwrap();
    let rendered = chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").unwrap();
    let yaml = &rendered[".github/workflows/chrono-ci.yml"];
    let start = yaml.find("  unit_example:\n").unwrap();
    let end = yaml[start..].find("  aggregate:\n").unwrap() + start;
    let job = &yaml[start..end];
    assert!(job.contains("needs.detect.outputs.unit_example == 'true'"));
    let positions: Vec<_> = [
        "Prepare registered caches",
        "Restore registered cache core.target",
        "Bootstrap registered tools",
        "Canonical harness check",
        "Save registered cache core.target",
        "Record original cache transport observations",
        "Preserve actual check evidence",
    ]
    .iter()
    .map(|name| job.find(name).unwrap())
    .collect();
    assert!(positions.windows(2).all(|p| p[0] < p[1]));
    assert!(job.contains("'check' '--unit' 'example'"));
    let seed = job
        .split("Prepare registered caches")
        .nth(1)
        .unwrap()
        .split("        run: |\n")
        .nth(1)
        .unwrap()
        .lines()
        .next()
        .unwrap();
    let seed_result = Command::new("/bin/bash")
        .args(["-e", "-c", seed])
        .output()
        .unwrap();
    assert!(seed_result.status.success());
    assert_eq!(seed_result.stdout, b"literal ' argument $(printf unsafe)");
    assert!(job.contains("steps.chrono_cache_plan.outputs.cache_0_key"));
    let core = job
        .split("Restore registered cache core.target")
        .nth(1)
        .unwrap()
        .split("      - name:")
        .next()
        .unwrap();
    assert!(core.contains("steps.chrono_cache_plan.outputs.cache_1_key"));
    assert!(job.contains("steps.chrono_cache_work.outcome == 'success'"));
    assert!(job.contains("continue-on-error: true"));
    assert!(job.contains("--plan-output '.chrono-harness/state/cache/host.build/plan.json'"));
    assert!(job.contains("CHRONO_CACHE_STEPS: ${{ toJSON(steps) }}"));
    assert_eq!(yaml.matches("Prepare registered caches").count(), 1);
    assert_eq!(yaml.matches("Save registered cache").count(), 2);
}

#[test]
fn completed_bootstrap_caches_can_be_saved_after_a_later_check_failure() {
    let mut value = source("examples/ci-host-job-gating/.chrono-harness/ci/units.json");
    value["persistent_cache"] = adoption("unit_example");
    value["persistent_cache"]["jobs"]["unit_example"]["save_after_bootstrap"] =
        json!(["core.target"]);
    let config = serde_json::from_value(value).unwrap();
    let rendered = chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").unwrap();
    let yaml: Value =
        serde_yaml_ng::from_str(&rendered[".github/workflows/chrono-ci.yml"]).unwrap();
    let steps = yaml["jobs"]["unit_example"]["steps"].as_array().unwrap();
    let bootstrap = steps
        .iter()
        .find(|s| s["name"] == "Bootstrap registered tools")
        .unwrap();
    assert_eq!(bootstrap["id"], "chrono_cache_bootstrap");
    let save = steps.iter().find(|s| s["id"] == "cache_0_save").unwrap();
    let condition = save["if"].as_str().unwrap();
    assert!(condition.contains("steps.chrono_cache_bootstrap.outcome == 'success'"));
    assert!(!condition.contains("steps.chrono_cache_work.outcome"));
    assert!(condition.contains("!cancelled()"));
    let check = steps
        .iter()
        .position(|s| s["name"] == "Canonical harness check")
        .unwrap();
    let saved = steps
        .iter()
        .position(|s| s["id"] == "cache_0_save")
        .unwrap();
    assert!(
        saved > check,
        "join the original work before saving shared outputs"
    );
    assert!(
        steps[check]["run"]
            .as_str()
            .unwrap()
            .contains("'check' '--unit' 'example'")
    );
}

#[test]
fn registered_cache_ids_fit_native_step_limits_without_truncating_cache_identity() {
    let mut value = source("examples/ci-host-job-gating/.chrono-harness/ci/units.json");
    value["persistent_cache"] = adoption("unit_example");
    let id = "a".repeat(128);
    value["persistent_cache"]["jobs"]["unit_example"]["caches"] = json!([id]);
    let config = serde_json::from_value(value).unwrap();
    let output = chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").unwrap();
    let yaml = &output[".github/workflows/chrono-ci.yml"];
    for line in yaml.lines() {
        if let Some(step) = line.trim().strip_prefix("id: ") {
            assert!(
                step.len() < 100,
                "native step exceeds the reference-name limit: {}",
                step.len()
            );
        }
    }
    assert!(yaml.contains("steps.chrono_cache_plan.outputs.cache_0_key"));
    assert!(yaml.contains(&format!("Restore registered cache {id}")));
    assert_eq!(yaml.matches("Restore registered cache").count(), 1);
    assert_eq!(yaml.matches("Save registered cache").count(), 1);
}

#[test]
fn actual_host_projections_fit_native_workflow_size() {
    let units = serde_json::from_value(source(".chrono-harness/ci/units.json")).unwrap();
    let release = serde_json::from_value(source(".chrono-harness/ci/release.json")).unwrap();
    let mut outputs = chrono_ci::units::render(&units, ".chrono-harness/ci/units.json").unwrap();
    outputs.insert(
        "release".into(),
        chrono_ci::release::render(&release).unwrap(),
    );
    for (name, output) in outputs {
        assert!(
            output.len() <= 500_000,
            "{name}: {} bytes exceed native workflow size",
            output.len()
        );
    }
    let yaml = chrono_ci::release::render(&release).unwrap();
    let expanded: Value = serde_yaml_ng::from_str(&yaml).unwrap();
    assert_eq!(
        expanded["jobs"].as_object().unwrap().len(),
        release.jobs.len()
    );
    for job in &release.jobs {
        let actual = &expanded["jobs"][&job.id];
        if !job.needs.is_empty() {
            assert_eq!(actual["needs"], json!(job.needs));
        }
        let downloads: Vec<_> = actual["steps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|step| {
                step["uses"]
                    .as_str()
                    .is_some_and(|v| v.starts_with("actions/download-artifact@"))
            })
            .collect();
        assert_eq!(downloads.len(), job.downloads.len(), "{}", job.id);
        for (step, download) in downloads.into_iter().zip(&job.downloads) {
            assert_eq!(step["with"]["path"], download.directory);
            assert_eq!(
                step["with"]["artifact-ids"],
                format!("${{{{ needs.{}.outputs.artifact_id }}}}", download.job)
            );
            assert_eq!(step["with"]["merge-multiple"], true);
            assert_eq!(
                step["if"],
                format!(
                    "${{{{ always() && needs.{}.outputs.artifact_id != '' }}}}",
                    download.job
                )
            );
        }
    }
}

#[test]
fn release_cache_wraps_only_the_named_original_unit() {
    let mut value = source(".chrono-harness/ci/release.json");
    value["persistent_cache"] = adoption("linux_build_runner");
    let config = serde_json::from_value(value).unwrap();
    let yaml = chrono_ci::release::render(&config).unwrap();
    let start = yaml.find("  linux_build_runner:\n").unwrap();
    let position = config
        .jobs
        .iter()
        .position(|j| j.id == "linux_build_runner")
        .unwrap();
    let end = config
        .jobs
        .get(position + 1)
        .map(|j| yaml.find(&format!("  {}:\n", j.id)).unwrap())
        .unwrap_or(yaml.len());
    let job = &yaml[start..end];
    assert!(job.contains("Prepare registered caches"));
    assert!(job.contains("--unit' 'build_runner'"));
    assert!(
        job.find("Restore registered cache").unwrap()
            < job.find("Run registered release command").unwrap()
    );
    assert!(
        job.find("Run registered release command").unwrap()
            < job.find("Save registered cache").unwrap()
    );
    assert_eq!(yaml.matches("Prepare registered caches").count(), 1);
}

#[test]
fn cache_adoption_rejects_unknown_jobs_unpinned_actions_and_empty_consumers() {
    let baseline = source("examples/ci-host-job-gating/.chrono-harness/ci/units.json");
    for (pointer, invalid) in [
        (
            "/persistent_cache/jobs",
            json!({"unknown": {"need":"existing build", "output_use":"build input", "prepare":["seed"],"consumer":"host.build","caches":["core.target"]}}),
        ),
        (
            "/persistent_cache/restore_action",
            json!("actions/cache/restore@v4"),
        ),
        ("/persistent_cache/jobs/unit_example/consumer", json!("")),
        ("/persistent_cache/jobs/unit_example/need", json!(" ")),
        ("/persistent_cache/jobs/unit_example/output_use", json!("")),
        ("/persistent_cache/jobs/unit_example/caches", json!([])),
        (
            "/persistent_cache/jobs/unit_example/caches",
            json!(["same", "same"]),
        ),
        ("/persistent_cache/config", json!("elsewhere.json")),
    ] {
        let mut value = baseline.clone();
        value["persistent_cache"] = adoption("unit_example");
        *value.pointer_mut(pointer).unwrap() = invalid;
        let config = serde_json::from_value(value).unwrap();
        assert!(
            chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").is_err(),
            "{pointer}"
        );
    }
    for ids in [
        json!(["unregistered"]),
        json!(["core.target", "core.target"]),
    ] {
        let mut value = baseline.clone();
        value["persistent_cache"] = adoption("unit_example");
        value["persistent_cache"]["jobs"]["unit_example"]["save_after_bootstrap"] = ids;
        let config = serde_json::from_value(value).unwrap();
        assert!(chrono_ci::units::render(&config, ".chrono-harness/ci/units.json").is_err());
    }
}

#[test]
fn release_init_and_generation_resolve_cache_consumers_before_writing() {
    let root = tempfile::tempdir().unwrap();
    let input = tempfile::NamedTempFile::new().unwrap();
    let mut value = source(".chrono-harness/ci/release.json");
    value["persistent_cache"] = adoption("linux_build_runner");
    fs::write(input.path(), serde_json::to_vec(&value).unwrap()).unwrap();
    let registry = root.path().join(".chrono-harness/cache.json");
    let adopted = root.path().join(".chrono-harness/ci/release.json");
    let workflow = root.path().join(value["workflow_path"].as_str().unwrap());
    let invalid = [
        json!({"schema":"chrono-cache/v1","require_primary_checkout":true,"caches":{"other":{"consumers":["host.build"]}}}),
        json!({"schema":"chrono-cache/v1","require_primary_checkout":true,"caches":{"core.target":{"consumers":["wrong.consumer"]}}}),
        json!({"schema":"chrono-cache/v1","caches":{"core.target":{"consumers":["host.build"]}}}),
    ];
    // Missing registry is also rejected, before either adoption or projection.
    assert!(chrono_ci::init(root.path(), input.path()).is_err());
    assert!(!adopted.exists());
    assert!(!workflow.exists());
    fs::create_dir_all(registry.parent().unwrap()).unwrap();
    for declaration in invalid {
        fs::write(&registry, serde_json::to_vec(&declaration).unwrap()).unwrap();
        assert!(chrono_ci::init(root.path(), input.path()).is_err());
        assert!(!adopted.exists());
        assert!(!workflow.exists());
    }
    let valid = json!({"schema":"chrono-cache/v1","require_primary_checkout":true,"caches":{"core.target":{"consumers":["host.build"]}}});
    fs::write(&registry, serde_json::to_vec(&valid).unwrap()).unwrap();
    assert!(chrono_ci::init(root.path(), input.path()).unwrap());
    assert!(!chrono_ci::generate(root.path(), ".chrono-harness/ci/release.json", true).unwrap());
    let before = fs::read(&workflow).unwrap();
    fs::remove_file(&registry).unwrap();
    assert!(chrono_ci::generate(root.path(), ".chrono-harness/ci/release.json", false).is_err());
    assert_eq!(before, fs::read(&workflow).unwrap());
}
