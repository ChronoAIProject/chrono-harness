#[path = "support/tools.rs"]
mod tools;
use chrono_ci::{generate, init, load, prepare};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};
use tools::fixture_git;

fn source() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.chrono-harness/ci/units.json");
    let provider: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let mut c = provider["collection"].clone();
    c["schema"] = "chrono-github-ci/v2".into();
    // This fixture exercises the historical v2 contract, without the host's newly adopted facts binding.
    c.as_object_mut().unwrap().remove("facts_config");
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
    let out = crate::tools::command(fixture_git())
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
    let host = crate::tools::temporary_host("explicit CI host ");
    let inputs = crate::tools::temporary_host("host λ ");
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
    let host = crate::tools::temporary_host("host λ ");
    let inputs = crate::tools::temporary_host("host λ ");
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
    let host = crate::tools::temporary_host("host λ ");
    let inputs = crate::tools::temporary_host("host λ ");
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
        "source-parent",
        "host-parent",
        "judge-collision",
    ] {
        let host = crate::tools::temporary_host("host λ ");
        let inputs = crate::tools::temporary_host("host λ ");
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
            "source-parent" => {
                c["initial_inventory"]["path"] =
                    ".chrono-harness/ci/github.json/initial.json".into()
            }
            "host-parent" => {
                c["initial_inventory"]["profile"]["host_config"] =
                    ".chrono-harness/ci/root inventory.json/config.json".into()
            }
            "judge-collision" => {
                c["initial_inventory"]["profile"]["judges"][0]["executable"] =
                    c["initial_inventory"]["path"].clone()
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
    let host = crate::tools::temporary_host("host λ ");
    let inputs = crate::tools::temporary_host("host λ ");
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

#[test]
fn generated_profile_runs_actual_root_registration_and_retains_failed_inventory() {
    let product = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for unregistered in [false, true] {
        let host = crate::tools::temporary_host("generated root host ");
        let inputs = crate::tools::temporary_host("host λ ");
        let root = host.path();
        let bin = root.join(".chrono-harness/bin");
        fs::create_dir_all(&bin).unwrap();
        for (project, name) in [
            ("runner", "chrono-harness"),
            ("judge-registration", "chrono-judge-registration"),
        ] {
            crate::tools::copied_fixture_input(
                root,
                &product.join(format!("crates/{project}/target/debug/{name}")),
                &bin.join(name),
            );
            fs::copy(
                product.join(format!("crates/{project}/target/debug/{name}")),
                bin.join(name),
            )
            .expect("build the explicitly registered production prerequisites");
        }
        let mut c = source();
        let binding = &mut c["initial_inventory"]["profile"]["judges"][0];
        binding["executable"] = ".chrono-harness/bin/chrono-judge-registration".into();
        binding["sha256"] =
            chrono_harness::sha256(&fs::read(bin.join("chrono-judge-registration")).unwrap())
                .into();
        let input = inputs.path().join("source.json");
        write(&input, &c);
        init(root, &input).unwrap();

        let read_host = |name: &str| -> Value {
            serde_json::from_slice(
                &fs::read(product.join(format!(".chrono-harness/{name}.json"))).unwrap(),
            )
            .unwrap()
        };
        let mut config = read_host("config");
        config["schema_version"] = json!(1);
        config.as_object_mut().unwrap().remove("facts_git");
        config["canonical_check"] = json!({"operation":"validate.delta", "argv":[]});
        config["tools"] = json!([]);
        config["semantic_fields"] = json!([]);
        config["environment"] = json!({"inherit":["PATH"], "values":{}, "inputs":[]});
        config["artifacts"] = json!([
            {"path":".chrono-harness/bin/","owner":"repository","kind":"executable","tracked":false},
            {"path":".chrono-harness/state/","owner":"repository","kind":"evidence","tracked":false}
        ]);
        config["canonical_check"]["argv"] = json!([
            ".chrono-harness/bin/chrono-harness",
            "check",
            "--config",
            ".chrono-harness/registry.json",
            "--base",
            "{base}",
            "--candidate",
            "{candidate}"
        ]);
        write(&root.join(".chrono-harness/registry.json"), &config);
        let mut workflow = read_host("workflow");
        workflow["stability"] = json!([]);
        workflow["integration"]["tests"] = json!([]);
        workflow["migrations"] = json!([]);
        workflow["retirements"] = json!([]);
        workflow["historical_profiles"] = json!([]);
        write(&root.join(".chrono-harness/workflow.json"), &workflow);
        write(
            &root.join(".chrono-harness/projects.json"),
            &json!({"schema_version":1,"status":"proposed","owners":["repository"],"projects":[],"scripts":[]}),
        );
        let mut normal_binding = c["initial_inventory"]["profile"]["judges"][0].clone();
        normal_binding["selector"] = "every-delta".into();
        normal_binding["modes"] = json!(["evaluate"]);
        normal_binding["argv"] = json!(["--protocol", "chrono-judge/v1"]);
        write(
            &root.join(".chrono-harness/judges.json"),
            &json!({"schema_version":1,"status":"proposed","judges":[normal_binding],"migration_validator":"inventory"}),
        );
        let files: Vec<_> = [".gitignore", ".chrono-harness/registry.json", ".chrono-harness/projects.json", ".chrono-harness/judges.json", ".chrono-harness/workflow.json", ".chrono-harness/FILEMAP.json", ".chrono-harness/ci/github.json", ".chrono-harness/ci/root inventory.json", ".github/workflows/chrono-ci.yml"]
            .into_iter().map(|path| json!({"path":path,"owner":"repository","surface":"documentation","cost":"unknown","edges":[]})).collect();
        write(
            &root.join(".chrono-harness/FILEMAP.json"),
            &json!({"schema_version":2,"status":"proposed","files":files,"project_edges":[],"test_costs":[],"execution_plans":{},"cost_models":{"unknown":{"cpu_ms":null,"wall_ms":null,"peak_rss_bytes":null,"io_bytes":null,"basis":"fixture unknown"}}}),
        );
        fs::write(
            root.join(".gitignore"),
            ".chrono-harness/bin/\n.chrono-harness/state/\n",
        )
        .unwrap();
        if unregistered {
            fs::write(root.join("unregistered.txt"), "must be reported").unwrap();
        }
        git(root, &["init", "-q", "-b", "dev"]);
        git(root, &["add", "."]);
        git(
            root,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "--no-gpg-sign",
                "-qm",
                "generated root",
            ],
        );
        let candidate = git(root, &["rev-parse", "HEAD"]);
        let adopted = load(&root.join(".chrono-harness/ci/github.json")).unwrap();
        let context = prepare(root, &adopted, "push", &json!({"ref":"refs/heads/dev","before":"0".repeat(40),"after":candidate,"created":true}), &candidate).unwrap();
        let mut argv: Vec<String> =
            serde_json::from_value(context["canonical_argv"].clone()).unwrap();
        assert_eq!(argv[3], c["initial_inventory"]["path"]);
        argv[3] = root.join(&argv[3]).to_str().unwrap().into();
        let output = crate::tools::command(root.join(&argv[0]))
            .args(&argv[1..])
            .current_dir(inputs.path())
            .env_remove("HOME")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(if unregistered { 1 } else { 0 }),
            "{} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            report["status"],
            if unregistered { "failed" } else { "complete" }
        );
        assert_eq!(report["governance"], "not-evaluated");
        assert!(report["base"].is_null() && report["delta"].is_null());
        assert_eq!(report["judges"][0]["state"], "executed");
        assert_eq!(
            report["judges"][0]["binding"]["sha256"],
            c["initial_inventory"]["profile"]["judges"][0]["sha256"]
        );
        assert_eq!(
            fs::read(root.join(".chrono-harness/state/initial-report.json")).unwrap(),
            output.stdout
        );
    }
}

#[test]
fn standalone_v4_preserves_explicit_initial_inventory_and_fixed_bare_entry() {
    let host = crate::tools::temporary_host("host λ ");
    let inputs = crate::tools::temporary_host("host λ ");
    let input = inputs.path().join("source.json");
    let mut c = source();
    c["schema"] = json!("chrono-github-ci/v4");
    c["facts_config"] = json!(".chrono-harness/registry.json");
    write(&input, &c);
    assert!(init(host.path(), &input).unwrap());
    assert!(!generate(host.path(), ".chrono-harness/ci/github.json", false).unwrap());
    assert!(!generate(host.path(), ".chrono-harness/ci/github.json", true).unwrap());
    let workflow =
        fs::read_to_string(host.path().join(c["workflow_path"].as_str().unwrap())).unwrap();
    assert!(workflow.contains("CHRONO_CHECK_SOURCE: ci"));
    assert!(workflow.contains("'.chrono-harness/bin/chrono-harness' 'check'\n"));
    assert!(!workflow.contains("--initial"));
    assert_eq!(
        serde_json::from_slice::<Value>(
            &fs::read(
                host.path()
                    .join(c["initial_inventory"]["path"].as_str().unwrap())
            )
            .unwrap()
        )
        .unwrap(),
        c["initial_inventory"]["profile"]
    );
}

mod initial_short;

#[test]
fn standalone_initial_migration_preserves_inventory_and_refuses_edits_or_weakening() {
    for case in [
        "valid",
        "inventory-change",
        "inventory-drift",
        "workflow-edit",
    ] {
        let host = crate::tools::temporary_host("host λ ");
        let inputs = crate::tools::temporary_host("host λ ");
        let input = inputs.path().join("source.json");
        let old = source();
        write(&input, &old);
        init(host.path(), &input).unwrap();
        let prior = ".chrono-harness/state/previous.json";
        fs::create_dir_all(host.path().join(".chrono-harness/state")).unwrap();
        fs::copy(
            host.path().join(".chrono-harness/ci/github.json"),
            host.path().join(prior),
        )
        .unwrap();
        let inventory = host
            .path()
            .join(old["initial_inventory"]["path"].as_str().unwrap());
        let before = fs::read(&inventory).unwrap();
        let mut next = old.clone();
        next["schema"] = json!("chrono-github-ci/v4");
        next["facts_config"] = json!(".chrono-harness/registry.json");
        if case == "inventory-change" {
            next["initial_inventory"]["profile"]["judges"][0]["sha256"] = json!("b".repeat(64));
        }
        if case == "inventory-drift" {
            fs::write(&inventory, b"host edit").unwrap();
        }
        if case == "workflow-edit" {
            fs::write(
                host.path().join(old["workflow_path"].as_str().unwrap()),
                b"host workflow edit",
            )
            .unwrap();
        }
        write(&host.path().join(".chrono-harness/ci/github.json"), &next);
        let workflow_before =
            fs::read(host.path().join(old["workflow_path"].as_str().unwrap())).unwrap();
        let result = chrono_ci::migrate::migrate(
            host.path(),
            prior,
            ".chrono-harness/ci/github.json",
            ".chrono-harness/ci/github.json",
        );
        if case == "valid" {
            assert!(result.is_ok(), "{result:?}");
            assert_eq!(fs::read(&inventory).unwrap(), before);
            assert!(!generate(host.path(), ".chrono-harness/ci/github.json", false).unwrap());
            assert!(!generate(host.path(), ".chrono-harness/ci/github.json", true).unwrap());
        } else {
            assert!(result.is_err(), "accepted {case}");
            assert_eq!(
                fs::read(host.path().join(old["workflow_path"].as_str().unwrap())).unwrap(),
                workflow_before
            );
        }
        assert_eq!(
            serde_json::from_slice::<Value>(
                &fs::read(host.path().join(".chrono-harness/ci/github.json")).unwrap()
            )
            .unwrap(),
            next
        );
    }
}
