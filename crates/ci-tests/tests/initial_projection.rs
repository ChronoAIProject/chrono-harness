use chrono_ci::{generate, init, load, prepare};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn source() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.chrono-harness/ci/github.json");
    let mut c: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    c["schema"] = "chrono-github-ci/v2".into();
    c["initial_inventory"] = json!({
        "path":".chrono-harness/ci/root inventory.json",
        "profile": {
            "schema":"chrono-initial-check/v1", "host_config":".chrono-harness/registry.json",
            "timeout_seconds":30, "stdout_limit_bytes":1048576,
            "judges":[{"id":"inventory","executable":".chrono-harness/bin/custom-judge",
                "version":"0.1.0","sha256":"a".repeat(64),
                "argv":["--protocol","chrono-initial-judge/v1"],
                "selector":"every-initial","modes":["inventory"],"after":[]}]
        }
    });
    c
}
fn write(path: &Path, value: &Value) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
}
fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}

#[test]
fn initial_projection_is_explicit_idempotent_and_used_by_the_initial_event() {
    let host = tempfile::Builder::new()
        .prefix("explicit CI host ")
        .tempdir()
        .unwrap();
    let inputs = tempfile::tempdir().unwrap();
    let input = inputs.path().join("source.json");
    let c = source();
    write(&input, &c);
    assert!(
        init(host.path(), &input)
            .expect("v2 must generate the explicitly registered initial profile")
    );
    let path = host
        .path()
        .join(c["initial_inventory"]["path"].as_str().unwrap());
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&path).unwrap()).unwrap(),
        c["initial_inventory"]["profile"]
    );
    let before = fs::metadata(&path).unwrap().modified().unwrap();
    assert!(!init(host.path(), &input).unwrap());
    assert_eq!(fs::metadata(&path).unwrap().modified().unwrap(), before);
    assert!(!generate(host.path(), ".chrono-harness/ci/github.json", true).unwrap());
    let workflow =
        fs::read_to_string(host.path().join(c["workflow_path"].as_str().unwrap())).unwrap();
    assert!(workflow.contains("'.chrono-harness/ci/root inventory.json'"));
    assert!(workflow.contains("'.chrono-harness/ci/check.json'"));
    git(host.path(), &["init", "-q", "-b", "dev"]);
    git(host.path(), &["add", "."]);
    git(
        host.path(),
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--no-gpg-sign",
            "-qm",
            "root",
        ],
    );
    let root_oid = git(host.path(), &["rev-parse", "HEAD"]);
    let adopted = load(&host.path().join(".chrono-harness/ci/github.json")).unwrap();
    let initial = prepare(
        host.path(),
        &adopted,
        "workflow_dispatch",
        &json!({"inputs":{"candidate":root_oid,"initial":true}}),
        &root_oid,
    )
    .unwrap();
    assert_eq!(initial["canonical_argv"][3], c["initial_inventory"]["path"]);
    assert!(initial["base"].is_null());
    let normal = prepare(
        host.path(),
        &adopted,
        "workflow_dispatch",
        &json!({"inputs":{"base":root_oid,"candidate":root_oid,"initial":false}}),
        &root_oid,
    )
    .unwrap();
    assert_eq!(normal["canonical_argv"][3], c["check_config"]);
    assert_eq!(normal["initial"], false);
}

#[test]
fn initial_projection_verification_detects_drift_without_repairing() {
    let host = tempfile::tempdir().unwrap();
    let inputs = tempfile::tempdir().unwrap();
    let input = inputs.path().join("source.json");
    let mut c = source();
    write(&input, &c);
    init(host.path(), &input).unwrap();
    let source_path = host.path().join(".chrono-harness/ci/github.json");
    let output_path = host
        .path()
        .join(c["initial_inventory"]["path"].as_str().unwrap());
    let old = fs::read(&output_path).unwrap();
    c["initial_inventory"]["profile"]["judges"][0]["sha256"] = "b".repeat(64).into();
    write(&source_path, &c);
    assert!(
        generate(host.path(), ".chrono-harness/ci/github.json", true)
            .unwrap_err()
            .contains("drift")
    );
    assert_eq!(fs::read(&output_path).unwrap(), old);
    assert!(generate(host.path(), ".chrono-harness/ci/github.json", false).unwrap());
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(output_path).unwrap()).unwrap(),
        c["initial_inventory"]["profile"]
    );
    assert!(!generate(host.path(), ".chrono-harness/ci/github.json", true).unwrap());
}

#[test]
fn initial_adoption_collision_is_preflighted_before_any_output_is_written() {
    let host = tempfile::tempdir().unwrap();
    let inputs = tempfile::tempdir().unwrap();
    let input = inputs.path().join("source.json");
    let c = source();
    write(&input, &c);
    let target = host
        .path()
        .join(c["initial_inventory"]["path"].as_str().unwrap());
    write(&target, &json!({"host":"owned"}));
    let before = fs::read(&target).unwrap();
    assert!(init(host.path(), &input).unwrap_err().contains("collision"));
    assert_eq!(fs::read(target).unwrap(), before);
    assert!(!host.path().join(".chrono-harness/ci/github.json").exists());
    assert!(
        !host
            .path()
            .join(c["workflow_path"].as_str().unwrap())
            .exists()
    );
}

#[test]
fn invalid_initial_sources_do_not_publish_partial_configuration() {
    for case in [
        "v1-extension",
        "v1-null",
        "v2-missing",
        "v2-null",
        "bad-profile",
        "digest",
        "cycle",
        "config-collision",
        "delta-collision",
        "host-config-collision",
        "state-output",
    ] {
        let host = tempfile::tempdir().unwrap();
        let inputs = tempfile::tempdir().unwrap();
        let input = inputs.path().join("source.json");
        let mut c = source();
        match case {
            "v1-extension" => c["schema"] = "chrono-github-ci/v1".into(),
            "v1-null" => {
                c["schema"] = "chrono-github-ci/v1".into();
                c["initial_inventory"] = Value::Null;
            }
            "v2-missing" => {
                c.as_object_mut().unwrap().remove("initial_inventory");
            }
            "v2-null" => c["initial_inventory"] = Value::Null,
            "bad-profile" => c["initial_inventory"]["profile"]["schema"] = "unknown/v1".into(),
            "digest" => c["initial_inventory"]["profile"]["judges"][0]["sha256"] = Value::Null,
            "cycle" => {
                c["initial_inventory"]["profile"]["judges"][0]["after"] = json!(["inventory"])
            }
            "config-collision" => {
                c["initial_inventory"]["path"] = ".chrono-harness/ci/github.json".into()
            }
            "delta-collision" => c["initial_inventory"]["path"] = c["check_config"].clone(),
            "host-config-collision" => {
                c["initial_inventory"]["profile"]["host_config"] =
                    c["initial_inventory"]["path"].clone()
            }
            "state-output" => {
                c["initial_inventory"]["path"] = ".chrono-harness/state/initial.json".into()
            }
            _ => unreachable!(),
        }
        write(&input, &c);
        assert!(init(host.path(), &input).is_err(), "accepted {case}");
        assert!(
            !host.path().join(".chrono-harness/ci/github.json").exists(),
            "{case}"
        );
        assert!(
            !host
                .path()
                .join(c["workflow_path"].as_str().unwrap())
                .exists(),
            "{case}"
        );
    }
}

#[test]
fn initial_projection_symlink_is_rejected_without_touching_the_target() {
    let host = tempfile::tempdir().unwrap();
    let inputs = tempfile::tempdir().unwrap();
    let input = inputs.path().join("source.json");
    let c = source();
    write(&input, &c);
    init(host.path(), &input).unwrap();
    let output = host
        .path()
        .join(c["initial_inventory"]["path"].as_str().unwrap());
    let target = inputs.path().join("host-owned");
    fs::write(&target, "preserved").unwrap();
    fs::remove_file(&output).unwrap();
    std::os::unix::fs::symlink(&target, &output).unwrap();
    for verify in [true, false] {
        assert!(generate(host.path(), ".chrono-harness/ci/github.json", verify).is_err());
        assert_eq!(fs::read_to_string(&target).unwrap(), "preserved");
    }
}
