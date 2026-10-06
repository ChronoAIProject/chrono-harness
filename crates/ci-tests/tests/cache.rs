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
fn gated_jobs_restore_before_bootstrap_and_save_without_replacing_the_check() {
    let mut value = source("examples/ci-host-job-gating/.chrono-harness/ci/units.json");
    value["persistent_cache"] = adoption("unit_example");
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
    assert!(job.contains("steps.chrono_cache_plan.outputs.cache_636f72652e746172676574_key"));
    assert!(job.contains("steps.chrono_cache_work.outcome == 'success'"));
    assert!(job.contains("continue-on-error: true"));
    assert_eq!(yaml.matches("Prepare registered caches").count(), 1);
    assert_eq!(yaml.matches("Save registered cache").count(), 1);
}

#[test]
fn registered_cache_ids_fit_native_step_limits_without_truncating_output_names() {
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
    let key = format!(
        "cache_{}",
        id.bytes().map(|b| format!("{b:02x}")).collect::<String>()
    );
    assert!(yaml.contains(&format!("steps.chrono_cache_plan.outputs.{key}_key")));
    assert_eq!(yaml.matches("Restore registered cache").count(), 1);
    assert_eq!(yaml.matches("Save registered cache").count(), 1);
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
