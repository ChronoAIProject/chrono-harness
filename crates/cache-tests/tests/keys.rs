#[path = "backend.rs"]
mod backend;
#[path = "recovery.rs"]
mod recovery;

use chrono_cache::{Config, prepare};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

struct Host(tempfile::TempDir);

impl Host {
    fn new() -> Self {
        let retained = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.chrono-harness/state/cache-test-failures");
        fs::create_dir_all(&retained).unwrap();
        Self(tempfile::tempdir_in(retained).unwrap())
    }

    fn path(&self) -> &Path {
        self.0.path()
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        if std::thread::panicking() {
            use std::io::Write;
            self.0.disable_cleanup(true);
            let _ = writeln!(
                std::io::stderr(),
                "Cache test original host: {}",
                self.path().display()
            );
        }
    }
}

fn fixture() -> (Host, Value) {
    let root = Host::new();
    fs::create_dir(root.path().join(".chrono-harness")).unwrap();
    fs::write(root.path().join("compiler"), b"compiler-one").unwrap();
    fs::write(root.path().join("source"), b"source-one").unwrap();
    let config = json!({"schema":"chrono-cache/v1","namespace":"fixture","artifact_registry":".chrono-harness/artifacts.json",
        "inputs":{
            "compiler":{"kind":"file","path":"compiler","presence":"present"},
            "source":{"kind":"file","path":"source","presence":"present"},
            "profile":{"kind":"literal","value":{"profile":"debug","incremental":true}},
            "other":{"kind":"file","path":"unavailable-other-input","presence":"present"}},
        "artifacts":{
            "target":{"owner":"project","path":"out/project","kind":"compilation","external":false},
            "other-target":{"owner":"other","path":"out/other","kind":"compilation","external":false}},
        "caches":{
            "project":{"owner":"project","producer":"build.project","consumers":["check.one","check.two"],
                "artifacts":["target"],"compatibility_inputs":["compiler","profile"],"source_inputs":["source"],"restore":"compatible"},
            "other":{"owner":"other","producer":"build.other","consumers":["check.other"],
                "artifacts":["other-target"],"compatibility_inputs":["compiler"],"source_inputs":["other"],"restore":"exact"}}});
    register_artifacts(root.path(), &config);
    (root, config)
}

fn register_artifacts(root: &Path, config: &Value) {
    let rows: Vec<_> = config["artifacts"].as_object().unwrap().values()
        .filter(|a| a["external"] == false)
        .map(|a| json!({"path":a["path"],"owner":a["owner"],"kind":"fixture-output","tracked":false})).collect();
    fs::write(
        root.join(".chrono-harness/artifacts.json"),
        serde_json::to_vec(&json!({"artifacts":rows})).unwrap(),
    )
    .unwrap();
}

fn plan(root: &Path, value: &Value, consumer: &str) -> Result<Value, String> {
    let config: Config = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    let result = prepare(root, &config, consumer);
    if let Err(error) = &result {
        let original = json!({"consumer":consumer,"config":value,"error":error});
        fs::write(
            root.join("failed-plan.json"),
            serde_json::to_vec(&original).unwrap(),
        )
        .unwrap();
    }
    result
}

#[test]
fn failed_test_retains_probe_originals_and_success_removes_its_host() {
    let (root, mut config) = fixture();
    let retained = root.path().to_path_buf();
    config["inputs"]["compiler"] = json!({"kind":"command","command":{
        "program":env!("CARGO_BIN_EXE_chrono-cache-test-probe"),"args":["exit"],"env":{},
        "timeout_seconds":5,"output_limit_bytes":4096},"inherit":[],"result":"stdout"});
    let failure = std::panic::catch_unwind(move || {
        plan(root.path(), &config, "check.one").unwrap();
    });
    assert!(failure.is_err());
    let original: Value =
        serde_json::from_slice(&fs::read(retained.join("failed-plan.json")).unwrap()).unwrap();
    assert_eq!(original["consumer"], "check.one");
    let error = original["error"].as_str().unwrap();
    let (_, path) = error.split_once("; original ").unwrap();
    let process: Value = serde_json::from_slice(&fs::read(retained.join(path)).unwrap()).unwrap();
    assert_eq!(process["exit_code"], 7);
    assert_eq!(process["failure"], Value::Null);
    assert_eq!(process["stderr"], "original probe failure\n");
    fs::remove_dir_all(retained).unwrap();

    let successful = Host::new();
    let removed = successful.path().to_path_buf();
    drop(successful);
    assert!(!removed.exists());
}

fn adopt_consumer_contract(root: &Path, config: &mut Value) {
    fs::write(
        root.join(".chrono-harness/operations.json"),
        br#"{"build":["build.project"],"other":"build.other"}"#,
    )
    .unwrap();
    config["schema"] = json!("chrono-cache/v2");
    config["consumer_operations"] = json!({
        "check.one":[{"registry":".chrono-harness/operations.json","pointer":"/build"}],
        "check.two":[{"registry":".chrono-harness/operations.json","pointer":"/build"}],
        "check.other":[{"registry":".chrono-harness/missing-other.json","pointer":"/other"}]
    });
}

#[test]
fn source_changes_reuse_compatible_domain_and_compiler_changes_do_not() {
    let (root, config) = fixture();
    let first = plan(root.path(), &config, "check.one").unwrap();
    fs::write(root.path().join("unregistered"), b"unrelated").unwrap();
    assert_eq!(first, plan(root.path(), &config, "check.one").unwrap());
    fs::write(root.path().join("source"), b"source-two").unwrap();
    let source = plan(root.path(), &config, "check.one").unwrap();
    assert_ne!(
        first["caches"]["project"]["key"],
        source["caches"]["project"]["key"]
    );
    assert_eq!(
        first["caches"]["project"]["restore_keys"],
        source["caches"]["project"]["restore_keys"]
    );
    fs::write(root.path().join("compiler"), b"compiler-two").unwrap();
    let compiler = plan(root.path(), &config, "check.one").unwrap();
    assert_ne!(
        source["caches"]["project"]["restore_keys"],
        compiler["caches"]["project"]["restore_keys"]
    );
    let prefix = compiler["caches"]["project"]["restore_keys"][0]
        .as_str()
        .unwrap();
    assert!(
        compiler["caches"]["project"]["key"]
            .as_str()
            .unwrap()
            .starts_with(prefix)
    );
}

#[test]
fn consumers_select_only_registered_inputs_and_never_fall_back_to_all() {
    let (root, mut config) = fixture();
    let first = plan(root.path(), &config, "check.one").unwrap();
    assert_eq!(first["caches"].as_object().unwrap().len(), 1);
    assert_eq!(first["inputs"].as_object().unwrap().len(), 3);
    assert_eq!(
        first["caches"],
        plan(root.path(), &config, "check.two").unwrap()["caches"]
    );
    assert!(
        plan(root.path(), &config, "missing")
            .unwrap_err()
            .contains("E_CACHE_CONSUMER")
    );
    assert!(
        plan(root.path(), &config, "check.other")
            .unwrap_err()
            .contains("unavailable-other-input")
    );
    config["caches"]["other"]["producer"] = json!("build.changed-other");
    assert_eq!(first, plan(root.path(), &config, "check.one").unwrap());
    assert_eq!(first["execution"], "not-started");
    assert_eq!(first["input_completeness_proven"], false);
    assert_eq!(first["caches"]["project"]["status"], "prepared-unrestored");
    assert_eq!(
        first["caches"]["project"]["handling"],
        "registered-build-required"
    );
}

#[test]
fn adding_a_consumer_does_not_invalidate_an_unchanged_producer_cache() {
    let (root, mut config) = fixture();
    adopt_consumer_contract(root.path(), &mut config);
    let first = plan(root.path(), &config, "check.one").unwrap();
    config["caches"]["project"]["consumers"] = json!(["check.one", "check.two", "check.new"]);
    config["consumer_operations"]["check.new"] = config["consumer_operations"]["check.one"].clone();
    assert_eq!(
        first["caches"],
        plan(root.path(), &config, "check.one").unwrap()["caches"]
    );
}

#[test]
fn adopted_consumer_contract_rejects_a_retired_producer_before_input_probes() {
    let (root, mut config) = fixture();
    adopt_consumer_contract(root.path(), &mut config);
    let source = root.path().join(".chrono-harness/operations.json");
    fs::write(
        &source,
        br#"{"build":["build.project"],"other":"build.other"}"#,
    )
    .unwrap();
    config["consumer_operations"] = json!({
        "check.one":[{"registry":".chrono-harness/operations.json","pointer":"/build"}],
        "check.two":[{"registry":".chrono-harness/operations.json","pointer":"/build"}],
        "check.other":[{"registry":".chrono-harness/missing-other.json","pointer":"/other"}]
    });
    let first = plan(root.path(), &config, "check.one").unwrap();
    assert_eq!(first["consumer_operations"]["status"], "declared");
    assert_eq!(
        first["consumer_operations"]["operations"],
        json!(["build.project"])
    );
    assert_eq!(
        first["consumer_operations"]["sources"][0]["operations"],
        json!(["build.project"])
    );
    // A retained cache subscription must fail as soon as the actual registered
    // plan no longer runs its producer; unavailable compiler inputs are later.
    fs::write(&source, br#"{"build":["build.replacement"]}"#).unwrap();
    fs::remove_file(root.path().join("compiler")).unwrap();
    let error = plan(root.path(), &config, "check.one").unwrap_err();
    assert!(error.contains("E_CACHE_CONSUMER"), "{error}");
    assert!(error.contains("build.project"), "{error}");
    assert!(error.contains("check.one"), "{error}");
}

#[test]
fn consumer_operation_references_reject_missing_ambiguous_and_nonoperation_values() {
    let (root, mut config) = fixture();
    adopt_consumer_contract(root.path(), &mut config);
    config["consumer_operations"] = json!({
        "check.one":[{"registry":".chrono-harness/operations.json","pointer":"/build"}],
        "check.two":[{"registry":".chrono-harness/operations.json","pointer":"/build"}],
        "check.other":[{"registry":".chrono-harness/operations.json","pointer":"/other"}]
    });
    let source = root.path().join(".chrono-harness/operations.json");
    for value in [
        json!(null),
        json!([]),
        json!(["build.project", "build.project"]),
        json!({"operation":"build.project"}),
        json!(["build.project", 1]),
    ] {
        fs::write(
            &source,
            serde_json::to_vec(&json!({"build":value})).unwrap(),
        )
        .unwrap();
        let error = plan(root.path(), &config, "check.one").unwrap_err();
        assert!(error.contains("E_CACHE_CONSUMER"), "{value}: {error}");
    }
    fs::write(&source, br#"{"build":"build.project"}"#).unwrap();
    assert!(plan(root.path(), &config, "check.one").is_ok());
    config["consumer_operations"]
        .as_object_mut()
        .unwrap()
        .remove("check.two");
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("check.two")
    );
}

#[test]
fn native_host_cache_consumers_resolve_original_operation_registrations() {
    let host = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut config: Value =
        serde_json::from_slice(&fs::read(host.join(".chrono-harness/cache.json")).unwrap())
            .unwrap();
    let root = tempfile::tempdir().unwrap();
    // Toolchain/key observation is tested separately. This fixture reads the
    // actual host's consumers and producer references without running platform probes.
    config["require_primary_checkout"] = json!(false);
    config["recover_failed_restores"] = json!(false);
    config["recover_unconfirmed_restores"] = json!(false);
    for input in config["inputs"].as_object_mut().unwrap().values_mut() {
        *input = json!({"kind":"literal","value":"fixture-only input observation"});
    }
    let mut sources = std::collections::BTreeSet::new();
    sources.insert(config["artifact_registry"].as_str().unwrap().to_owned());
    for refs in config["consumer_operations"].as_object().unwrap().values() {
        for source in refs.as_array().unwrap() {
            sources.insert(source["registry"].as_str().unwrap().to_owned());
        }
    }
    for source in sources {
        let path = root.path().join(&source);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::copy(host.join(source), path).unwrap();
    }
    for consumer in config["consumer_operations"].as_object().unwrap().keys() {
        let prepared =
            plan(root.path(), &config, consumer).unwrap_or_else(|e| panic!("{consumer}: {e}"));
        assert_eq!(prepared["consumer_operations"]["status"], "declared");
    }
    // Reintroduce one real retired routes subscription: fail before transport.
    config["caches"]["check.judge-registration"]["consumers"]
        .as_array_mut()
        .unwrap()
        .push(json!("check.unit_judge-routes"));
    let error = plan(root.path(), &config, "check.unit_judge-routes").unwrap_err();
    assert!(error.contains("build.judge-registration"), "{error}");
}

#[test]
fn a_consumer_registry_cannot_be_restored_as_a_cache_artifact() {
    let (root, mut config) = fixture();
    adopt_consumer_contract(root.path(), &mut config);
    config["artifacts"]["target"]["path"] = json!(".chrono-harness/operations.json");
    register_artifacts(root.path(), &config);
    let error = plan(root.path(), &config, "check.one").unwrap_err();
    assert!(
        error.contains("E_CACHE_PATH") && error.contains("consumer registry"),
        "{error}"
    );
}

#[test]
fn transport_report_distinguishes_restore_evidence_and_unconfirmed_save_outcome() {
    let (root, config) = fixture();
    let prepared = plan(root.path(), &config, "check.one").unwrap();
    let key = prepared["caches"]["project"]["key"].as_str().unwrap();
    let steps = json!({
        "cache_0_restore":{"outcome":"success","outputs":{"cache-matched-key":key}},
        "cache_0_save":{"outcome":"skipped","outputs":{}},
        "build":{"outcome":"success"},"work":{"outcome":"failure"}
    });
    let report = chrono_cache::transport_report(&prepared, &steps, "work", Some("build")).unwrap();
    assert_eq!(report["caches"]["project"]["restore"]["status"], "exact");
    assert_eq!(
        report["caches"]["project"]["save"]["status"],
        "not-attempted"
    );
    assert_eq!(report["work"]["outcome"], "failure");
    assert_eq!(report["bootstrap"]["outcome"], "success");
    assert_eq!(report["verdict"], "not-a-judgment");
    for (matched, outcome, expected) in [
        ("", "success", "miss-or-unavailable"),
        ("", "failure", "error"),
        ("", "skipped", "not-attempted"),
    ] {
        let mut steps = steps.clone();
        steps["cache_0_restore"] =
            json!({"outcome":outcome,"outputs":{"cache-matched-key":matched}});
        steps["cache_0_save"]["outcome"] = json!("success");
        let report =
            chrono_cache::transport_report(&prepared, &steps, "work", Some("build")).unwrap();
        assert_eq!(report["caches"]["project"]["restore"]["status"], expected);
        assert_eq!(report["caches"]["project"]["save"]["status"], "unconfirmed");
        assert_eq!(report["caches"]["project"]["save"]["confirmed"], false);
        assert!(
            report["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|w| w["code"] == "W_CACHE_SAVE_UNCONFIRMED")
        );
    }
    let prefix = prepared["caches"]["project"]["restore_keys"][0]
        .as_str()
        .unwrap();
    let mut steps = steps;
    steps["cache_0_restore"]["outputs"]["cache-matched-key"] =
        json!(format!("{prefix}older-source"));
    assert_eq!(
        chrono_cache::transport_report(&prepared, &steps, "work", None).unwrap()["caches"]["project"]
            ["restore"]["status"],
        "compatible"
    );
    steps["cache_0_restore"]["outputs"]["cache-matched-key"] = json!("different-domain");
    let report = chrono_cache::transport_report(&prepared, &steps, "work", None).unwrap();
    assert_eq!(
        report["caches"]["project"]["restore"]["status"],
        "incompatible"
    );
    assert_eq!(
        report["warnings"][0]["code"],
        "W_CACHE_RESTORE_INCOMPATIBLE"
    );
}

#[test]
fn provider_cli_preserves_plan_probe_and_native_failure_observations_in_upload_directory() {
    let (root, mut config) = fixture();
    config["inputs"]["compiler"] = json!({"kind":"command","command":{
        "program":env!("CARGO_BIN_EXE_chrono-cache-test-probe"),"args":["echo","actual compiler"],"env":{},
        "timeout_seconds":5,"output_limit_bytes":4096},"inherit":[],"result":"stdout"});
    fs::write(
        root.path().join(".chrono-harness/cache.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../cache/target/debug/chrono-cache");
    let directory = ".chrono-harness/cache/release-results/unit/cache-observations/";
    let plan_path = ".chrono-harness/state/cache/check.one/plan.json".to_owned();
    let run = Command::new(&binary)
        .args([
            "plan",
            "--host-root",
            root.path().to_str().unwrap(),
            "--config",
            ".chrono-harness/cache.json",
            "--consumer",
            "check.one",
            "--plan-output",
            &plan_path,
        ])
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let prepared: Value = serde_json::from_slice(&run.stdout).unwrap();
    assert_eq!(
        prepared,
        serde_json::from_slice::<Value>(&fs::read(root.path().join(&plan_path)).unwrap()).unwrap()
    );
    let original = prepared["inputs"]["compiler"]["original"].as_str().unwrap();
    assert!(original.starts_with(".chrono-harness/state/cache/check.one/"));
    assert!(
        !root.path().join(directory).exists(),
        "preparation must not create release output"
    );
    let probe: Value =
        serde_json::from_slice(&fs::read(root.path().join(original)).unwrap()).unwrap();
    assert_eq!(probe["exit_code"], 0);
    let steps = json!({"cache_0_restore":{"outcome":"success","outputs":{}},"cache_0_save":{"outcome":"success","outputs":{}},"work":{"outcome":"failure"}});
    let result = Command::new(&binary)
        .args([
            "report",
            "--host-root",
            root.path().to_str().unwrap(),
            "--plan",
            &plan_path,
            "--report-directory",
            directory,
            "--steps-env",
            "CACHE_TEST_STEPS",
            "--work",
            "work",
        ])
        .env("CACHE_TEST_STEPS", serde_json::to_string(&steps).unwrap())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stderr).contains("W_CACHE_SAVE_UNCONFIRMED"));
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    let original = report["report"]["path"].as_str().unwrap();
    assert!(original.starts_with(directory));
    assert_eq!(
        report["observation"],
        serde_json::from_slice::<Value>(&fs::read(root.path().join(original)).unwrap()).unwrap()
    );
    let producer = &report["observation"]["producer"];
    assert_eq!(
        producer["executable"],
        json!(fs::canonicalize(&binary).unwrap())
    );
    assert_eq!(producer["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(
        producer["sha256"],
        format!("{:x}", Sha256::digest(fs::read(&binary).unwrap()))
    );
    let steps_path = report["observation"]["native_steps"]["path"]
        .as_str()
        .unwrap();
    assert!(steps_path.starts_with(directory));
    assert_eq!(
        steps,
        serde_json::from_slice::<Value>(&fs::read(root.path().join(steps_path)).unwrap()).unwrap()
    );
    assert_eq!(report["observation"]["work"]["outcome"], "failure");
    let probes = report["observation"]["probe_uploads"].as_array().unwrap();
    assert_eq!(probes.len(), 1);
    let copied = probes[0]["upload"]["path"].as_str().unwrap();
    assert!(copied.starts_with(directory));
    assert_eq!(
        probe,
        serde_json::from_slice::<Value>(&fs::read(root.path().join(copied)).unwrap()).unwrap()
    );
    for (steps, expected, warning) in [
        (json!({"work":{"outcome":"failure"}}), "not-requested", None),
        (
            json!({"cache_0_save":{"outcome":"success"},"work":{"outcome":"success"}}),
            "unconfirmed",
            Some("W_CACHE_UNREGISTERED_SAVE"),
        ),
    ] {
        let out = Command::new(&binary)
            .args([
                "report",
                "--host-root",
                root.path().to_str().unwrap(),
                "--plan",
                &plan_path,
                "--report-directory",
                ".chrono-harness/state/cache/check.one/",
                "--steps-env",
                "CACHE_TEST_STEPS",
                "--work",
                "work",
                "--save-caches",
                "[]",
            ])
            .env("CACHE_TEST_STEPS", serde_json::to_string(&steps).unwrap())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let row: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(
            row["observation"]["caches"]["project"]["save"]["status"],
            expected
        );
        assert_eq!(
            row["observation"]["caches"]["project"]["save"]["requested"],
            false
        );
        if let Some(code) = warning {
            assert!(
                row["observation"]["warnings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|w| w["code"] == code)
            );
        }
    }
    for saves in [r#"["unknown"]"#, r#"["project","project"]"#] {
        let out = Command::new(&binary)
            .args([
                "report",
                "--host-root",
                root.path().to_str().unwrap(),
                "--plan",
                &plan_path,
                "--report-directory",
                directory,
                "--steps-env",
                "CACHE_TEST_STEPS",
                "--work",
                "work",
                "--save-caches",
                saves,
            ])
            .env("CACHE_TEST_STEPS", "{}")
            .output()
            .unwrap();
        assert!(!out.status.success());
    }
}

#[test]
fn compatibility_binds_profile_producer_artifacts_owner_and_namespace() {
    let (root, config) = fixture();
    let first = plan(root.path(), &config, "check.one").unwrap();
    for pointer in [
        "/namespace",
        "/inputs/profile/value/profile",
        "/caches/project/producer",
        "/artifacts/target/path",
    ] {
        let mut changed = config.clone();
        *changed.pointer_mut(pointer).unwrap() = json!("changed");
        register_artifacts(root.path(), &changed);
        let next = plan(root.path(), &changed, "check.one").unwrap();
        assert_ne!(
            first["caches"]["project"]["restore_keys"], next["caches"]["project"]["restore_keys"],
            "{pointer}"
        );
    }
    let mut changed = config.clone();
    changed["caches"]["project"]["owner"] = json!("changed-owner");
    changed["artifacts"]["target"]["owner"] = json!("changed-owner");
    register_artifacts(root.path(), &changed);
    assert_ne!(
        first["caches"],
        plan(root.path(), &changed, "check.one").unwrap()["caches"]
    );
}

#[test]
fn absent_inputs_are_explicit_and_empty_files_are_present() {
    let (root, mut config) = fixture();
    fs::remove_file(root.path().join("source")).unwrap();
    assert!(plan(root.path(), &config, "check.one").is_err());
    config["inputs"]["source"]["presence"] = json!("absent");
    let absent = plan(root.path(), &config, "check.one").unwrap();
    assert_eq!(
        absent["inputs"]["source"]["observation"]["presence"],
        "absent"
    );
    fs::write(root.path().join("source"), b"").unwrap();
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("expected absent")
    );
    config["inputs"]["source"]["presence"] = json!("present");
    let present = plan(root.path(), &config, "check.one").unwrap();
    assert_eq!(present["inputs"]["source"]["observation"]["length"], 0);
    assert_ne!(absent["caches"], present["caches"]);
}

#[test]
fn executable_candidates_require_exact_keys_and_still_require_builds() {
    let (root, mut config) = fixture();
    config["artifacts"]["target"]["kind"] = json!("executable-candidate");
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("exact restoration")
    );
    config["caches"]["project"]["restore"] = json!("exact");
    let result = plan(root.path(), &config, "check.one").unwrap();
    assert_eq!(result["caches"]["project"]["restore_keys"], json!([]));
    assert_eq!(
        result["caches"]["project"]["handling"],
        "registered-build-required"
    );
}

#[test]
fn registration_rejects_missing_ambiguous_and_cross_owner_references() {
    let (root, config) = fixture();
    for (pointer, value) in [
        ("/caches/project/source_inputs", json!(["unknown"])),
        ("/caches/project/source_inputs", json!(["source", "source"])),
        ("/caches/project/source_inputs", json!(["compiler"])),
        ("/caches/project/artifacts", json!(["missing"])),
        ("/caches/project/artifacts", json!(["other-target"])),
        (
            "/caches/project/consumers",
            json!(["check.one", "check.one"]),
        ),
        ("/caches/project/compatibility_inputs", json!([])),
        ("/artifacts/target/external", json!(true)),
    ] {
        let mut changed = config.clone();
        *changed.pointer_mut(pointer).unwrap() = value;
        assert!(
            plan(root.path(), &changed, "check.one")
                .unwrap_err()
                .contains("E_CACHE_CONFIG"),
            "{pointer}"
        );
    }
    let mut unknown = config.clone();
    unknown["infer_dependencies"] = json!(true);
    assert!(serde_json::from_value::<Config>(unknown).is_err());
}

#[test]
fn cache_paths_exclude_evidence_metadata_sources_and_overlapping_outputs() {
    let (root, config) = fixture();
    for path in [
        ".git",
        ".git/objects",
        ".chrono-harness",
        ".chrono-harness/state",
        ".chrono-harness/state/check.json",
        "source",
        "out/../source",
        "out/*",
    ] {
        let mut changed = config.clone();
        changed["artifacts"]["target"]["path"] = json!(path);
        register_artifacts(root.path(), &changed);
        assert!(
            plan(root.path(), &changed, "check.one")
                .unwrap_err()
                .contains("E_CACHE_PATH"),
            "{path}"
        );
    }
    let mut changed = config.clone();
    changed["caches"]["project"]["artifacts"] = json!(["target", "overlap"]);
    changed["artifacts"]["overlap"] = json!({"owner":"project","path":"out/project/nested","kind":"compilation","external":false});
    register_artifacts(root.path(), &changed);
    assert!(
        plan(root.path(), &changed, "check.one")
            .unwrap_err()
            .contains("overlap")
    );
}

#[test]
fn host_artifacts_require_exact_existing_registration_and_external_caches_are_explicit() {
    let (root, mut config) = fixture();
    let catalog = root.path().join(".chrono-harness/artifacts.json");
    for rows in [
        json!([]),
        json!([{"path":"out/project","owner":"project","tracked":true}]),
        json!([{"path":"out/project","owner":"project","tracked":false,"kind":"execution-evidence"}]),
        json!([{"path":"out/project","owner":"wrong-owner","tracked":false}]),
        json!([{"path":"out/project","owner":"project","tracked":false}, {"path":"out/project","owner":"project","tracked":false}]),
    ] {
        fs::write(
            &catalog,
            serde_json::to_vec(&json!({"artifacts":rows})).unwrap(),
        )
        .unwrap();
        assert!(
            plan(root.path(), &config, "check.one")
                .unwrap_err()
                .contains("E_CACHE_ARTIFACT")
        );
    }
    register_artifacts(root.path(), &config);
    let external = tempfile::tempdir().unwrap();
    config["artifacts"]["target"]["path"] = json!(fs::canonicalize(external.path()).unwrap());
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("external/path")
    );
    config["artifacts"]["target"]["external"] = json!(true);
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("dependency caches")
    );
    config["artifacts"]["target"]["kind"] = json!("dependencies");
    assert!(plan(root.path(), &config, "check.one").is_ok());
    config["artifacts"]["target"]["path"] = json!(fs::canonicalize(root.path()).unwrap());
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("E_CACHE_PATH")
    );
}

#[cfg(unix)]
#[test]
fn file_and_cache_aliases_do_not_silently_expand_registered_paths() {
    let (root, config) = fixture();
    fs::rename(root.path().join("source"), root.path().join("real-source")).unwrap();
    std::os::unix::fs::symlink("real-source", root.path().join("source")).unwrap();
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("symlink")
    );
    fs::remove_file(root.path().join("source")).unwrap();
    fs::rename(root.path().join("real-source"), root.path().join("source")).unwrap();
    std::os::unix::fs::symlink(".chrono-harness", root.path().join("out")).unwrap();
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("symlink")
    );
}

#[test]
fn cli_observes_only_explicit_child_environment_without_exposing_values() {
    let (root, mut config) = fixture();
    config["inputs"]["profile"] =
        json!({"kind":"environment","name":"CHRONO_CACHE_TEST_PROFILE","presence":"present"});
    let path = root.path().join(".chrono-harness/cache.json");
    fs::write(path, serde_json::to_vec(&config).unwrap()).unwrap();
    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../cache/target/debug/chrono-cache");
    let run = |value: Option<&str>| {
        let mut command = Command::new(&binary);
        command.args([
            "plan",
            "--host-root",
            root.path().to_str().unwrap(),
            "--config",
            ".chrono-harness/cache.json",
            "--consumer",
            "check.one",
        ]);
        command.env_remove("CHRONO_CACHE_TEST_PROFILE");
        if let Some(value) = value {
            command.env("CHRONO_CACHE_TEST_PROFILE", value);
        }
        command.output().unwrap()
    };
    let missing = run(None);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("E_CACHE_INPUT"));
    let first = run(Some("first-private-value"));
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(!String::from_utf8_lossy(&first.stdout).contains("first-private-value"));
    let first: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(first["schema"], "chrono-cache-plan/v1");
    assert_eq!(first["consumer"], "check.one");
    assert_eq!(first["execution"], "not-started");
    let next = run(Some("second-private-value"));
    assert!(next.status.success());
    let next: Value = serde_json::from_slice(&next.stdout).unwrap();
    assert_ne!(
        first["caches"]["project"]["restore_keys"],
        next["caches"]["project"]["restore_keys"]
    );
}

#[test]
fn consumer_registration_and_restore_policy_are_bound_without_list_order() {
    let (root, mut config) = fixture();
    let first = plan(root.path(), &config, "check.one").unwrap();
    config["caches"]["project"]["consumers"] = json!(["check.two", "check.one"]);
    assert_eq!(first, plan(root.path(), &config, "check.one").unwrap());
    config["caches"]["project"]["consumers"] = json!(["check.one", "check.three"]);
    let changed = plan(root.path(), &config, "check.one").unwrap();
    assert_ne!(
        first["caches"]["project"]["key"],
        changed["caches"]["project"]["key"]
    );
    config["caches"]["project"]["restore"] = json!("exact");
    let exact = plan(root.path(), &config, "check.one").unwrap();
    assert_ne!(
        changed["caches"]["project"]["key"],
        exact["caches"]["project"]["key"]
    );
    assert_eq!(exact["caches"]["project"]["restore_keys"], json!([]));
}

#[test]
fn cli_rejects_unknown_actions_and_ambiguous_options() {
    for args in [
        vec![],
        vec!["unknown"],
        vec!["plan", "--unknown", "x"],
        vec!["plan", "--host-root", ".", "--host-root", "."],
        vec!["plan", "--consumer"],
    ] {
        let args = args.into_iter().map(str::to_string).collect::<Vec<_>>();
        assert!(chrono_cache::dispatch(&args).is_err());
    }
}

#[test]
fn github_outputs_preserve_selected_paths_and_never_publish_a_failed_plan() {
    let (root, mut config) = fixture();
    config["caches"]["other"]["consumers"] = json!(["check.one"]);
    fs::write(root.path().join("unavailable-other-input"), b"other source").unwrap();
    config["artifacts"]["target"]["path"] = json!("outputs with spaces/target");
    register_artifacts(root.path(), &config);
    fs::write(
        root.path().join(".chrono-harness/cache.json"),
        serde_json::to_vec(&config).unwrap(),
    )
    .unwrap();
    let output = root.path().join("github-output");
    fs::write(&output, "existing=value\n").unwrap();
    let binary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../cache/target/debug/chrono-cache");
    let invoke = |consumer: &str| {
        Command::new(&binary)
            .args([
                "plan",
                "--host-root",
                root.path().to_str().unwrap(),
                "--config",
                ".chrono-harness/cache.json",
                "--consumer",
                consumer,
                "--github-output",
                output.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    };
    let result = invoke("check.one");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let prepared: Value = serde_json::from_slice(&result.stdout).unwrap();
    let contents = fs::read_to_string(&output).unwrap();
    assert!(contents.starts_with("existing=value\n"));
    let mut values = std::collections::BTreeMap::new();
    let mut lines = contents.lines().skip(1);
    while let Some(header) = lines.next() {
        let (name, delimiter) = header.split_once("<<").unwrap();
        let mut value = Vec::new();
        loop {
            let line = lines.next().unwrap();
            if line == delimiter {
                break;
            }
            value.push(line);
        }
        assert!(values.insert(name, value.join("\n")).is_none());
    }
    assert_eq!(values.len(), 6);
    assert_eq!(values["cache_0_key"], prepared["caches"]["other"]["key"]);
    assert_eq!(values["cache_1_key"], prepared["caches"]["project"]["key"]);
    assert_eq!(values["cache_1_paths"], "outputs with spaces/target");
    assert_eq!(
        values["cache_1_restore"],
        prepared["caches"]["project"]["restore_keys"][0]
    );
    assert_eq!(prepared["execution"], "not-started");
    let failed = invoke("missing");
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("E_CACHE_CONSUMER"));
    assert_eq!(fs::read_to_string(&output).unwrap(), contents);
}

#[test]
fn registered_probe_binds_actual_selected_compiler_bytes_with_stable_keys() {
    let (root, mut config) = fixture();
    let compiler = root.path().join("compiler");
    config["inputs"]["compiler"] = json!({"kind":"command", "command":{
        "program":env!("CARGO_BIN_EXE_chrono-cache-test-probe"),
        "args":["file",compiler],"env":{},"timeout_seconds":2,"output_limit_bytes":4096},
        "inherit":[],"result":"file-path"});
    let first = plan(root.path(), &config, "check.one").unwrap();
    assert_eq!(
        first,
        plan(root.path(), &config, "check.one").unwrap(),
        "evidence times must not change keys"
    );
    fs::write(&compiler, b"compiler-two").unwrap();
    let next = plan(root.path(), &config, "check.one").unwrap();
    assert_ne!(
        first["caches"]["project"]["restore_keys"],
        next["caches"]["project"]["restore_keys"]
    );
    let observed = &first["inputs"]["compiler"]["observation"];
    assert_eq!(observed["exit_code"], 0);
    assert_eq!(observed["file"]["length"], b"compiler-one".len());
    assert!(observed["executable_sha256"].as_str().unwrap().len() == 64);
}

#[test]
fn named_probe_uses_only_its_declared_path_and_records_the_bound_executable() {
    let (root, mut config) = fixture();
    let tools = root.path().join("tools");
    fs::create_dir(&tools).unwrap();
    let executable = tools.join("registered-probe");
    fs::copy(env!("CARGO_BIN_EXE_chrono-cache-test-probe"), &executable).unwrap();
    config["inputs"]["compiler"] = json!({"kind":"command", "command":{
        "program":"registered-probe", "args":["echo","actual compiler"],
        "env":{"PATH":tools},"timeout_seconds":2,"output_limit_bytes":4096},
        "inherit":[],"result":"stdout"});
    let prepared = plan(root.path(), &config, "check.one").unwrap();
    let observed = Path::new(
        prepared["inputs"]["compiler"]["observation"]["executable"]
            .as_str()
            .unwrap(),
    );
    assert!(observed.is_absolute());
    assert_eq!(
        fs::canonicalize(observed).unwrap(),
        fs::canonicalize(executable).unwrap()
    );
    config["inputs"]["compiler"]["command"]["env"] = json!({});
    assert!(
        plan(root.path(), &config, "check.one").is_err(),
        "a missing declared PATH must not fall back to ambient tools"
    );
}

#[test]
fn failed_and_bounded_probes_retain_original_results_and_stop_planning() {
    for (mode, limit, timeout) in [("exit", 4096, 5), ("sleep", 4096, 1), ("flood", 64, 5)] {
        let (root, mut config) = fixture();
        config["inputs"]["compiler"] = json!({"kind":"command","command":{
            "program":env!("CARGO_BIN_EXE_chrono-cache-test-probe"),"args":[mode],"env":{},
            "timeout_seconds":timeout,"output_limit_bytes":limit},"inherit":[],"result":"stdout"});
        let error = plan(root.path(), &config, "check.one").unwrap_err();
        assert!(error.contains("E_CACHE_PROBE"), "{error}");
        let files = fs::read_dir(root.path().join(".chrono-harness/state/cache-probes"))
            .unwrap()
            .map(|v| v.unwrap().path())
            .collect::<Vec<_>>();
        assert_eq!(files.len(), 1);
        let evidence: Value = serde_json::from_slice(&fs::read(&files[0]).unwrap()).unwrap();
        assert!(error.contains(files[0].file_name().unwrap().to_str().unwrap()));
        if mode == "exit" {
            assert_eq!(evidence["exit_code"], 7, "{evidence}");
            assert_eq!(evidence["stderr"], "original probe failure\n");
        } else {
            let expected = if mode == "sleep" {
                "process timed out"
            } else {
                "process output limit exceeded"
            };
            assert_eq!(evidence["failure"], expected, "{evidence}");
        }
    }
}

#[test]
fn registered_source_keys_follow_only_explicit_transitive_filemap_edges() {
    let (root, mut config) = fixture();
    config["inputs"]["source"] = json!({"kind":"registered-files","registry":".chrono-harness/FILEMAP.json","node":"project:product","edges":["compile"]});
    let mut registry = json!({"schema_version":2,"project_edges":[
        {"from":"project:dependency","to":"project:product","kind":"compile"},
        {"from":"project:unrelated","to":"project:product","kind":"test-execution"}],
        "files":[
            {"path":"source","edges":[{"kind":"compile","to":"project:product"}]},
            {"path":"dependency","edges":[{"kind":"compile","to":"project:dependency"}]},
            {"path":"unavailable","edges":[{"kind":"compile","to":"project:unrelated"}]}]});
    fs::write(root.path().join("dependency"), b"first dependency").unwrap();
    let invoke = |registry: &Value| {
        fs::write(
            root.path().join(".chrono-harness/FILEMAP.json"),
            serde_json::to_vec(registry).unwrap(),
        )
        .unwrap();
        fs::write(
            root.path().join(".chrono-harness/cache.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        let binary =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../cache/target/debug/chrono-cache");
        let result = Command::new(binary)
            .args([
                "plan",
                "--host-root",
                root.path().to_str().unwrap(),
                "--config",
                ".chrono-harness/cache.json",
                "--consumer",
                "check.one",
            ])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        serde_json::from_slice::<Value>(&result.stdout).unwrap()
    };
    let first = invoke(&registry);
    assert_eq!(
        first["inputs"]["source"]["observation"]["files"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    registry["files"][2]["cost"] = json!("unrelated membership update");
    assert_eq!(first, invoke(&registry));
    fs::write(root.path().join("dependency"), b"next dependency").unwrap();
    let changed = invoke(&registry);
    assert_ne!(
        first["caches"]["project"]["key"],
        changed["caches"]["project"]["key"]
    );
    assert_eq!(
        first["caches"]["project"]["restore_keys"],
        changed["caches"]["project"]["restore_keys"]
    );
    registry["project_edges"][0]["kind"] = json!("runtime-input");
    let detached = invoke(&registry);
    assert_eq!(
        detached["inputs"]["source"]["observation"]["files"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
    assert_ne!(
        changed["caches"]["project"]["key"],
        detached["caches"]["project"]["key"]
    );
}

#[test]
fn primary_checkout_transport_refuses_linked_worktrees_before_restoration() {
    let (root, mut config) = fixture();
    config["require_primary_checkout"] = json!(true);
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("E_CACHE_OWNERSHIP")
    );
    let git = |args: &[&str]| {
        let result = Command::new("/usr/bin/git")
            .args(args)
            .current_dir(root.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.invalid",
        "commit",
        "-qm",
        "fixture",
    ]);
    assert!(plan(root.path(), &config, "check.one").is_ok());
    let linked = tempfile::tempdir().unwrap();
    git(&[
        "worktree",
        "add",
        "--detach",
        linked.path().to_str().unwrap(),
        "HEAD",
    ]);
    assert!(
        plan(linked.path(), &config, "check.one")
            .unwrap_err()
            .contains("E_CACHE_OWNERSHIP")
    );
}

fn host_cache_fixture(consumers: &[&str]) -> (Host, Value) {
    let host = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let mut config: Value =
        serde_json::from_slice(&fs::read(host.join(".chrono-harness/cache.json")).unwrap())
            .unwrap();
    let root = Host::new();
    fs::create_dir_all(root.path().join(".chrono-harness/ci")).unwrap();
    config["require_primary_checkout"] = json!(false);
    config["recover_failed_restores"] = json!(false);
    config["recover_unconfirmed_restores"] = json!(false);
    // Vary actual host policy files while keeping unrelated compiler/source
    // observations fixed; this contract fixture needs no compiler or SDK probes.
    for (id, input) in config["inputs"].as_object_mut().unwrap() {
        if input["path"] != ".chrono-harness/ci/check.json"
            && input["path"] != ".chrono-harness/ci/bootstrap-core.json"
        {
            *input = json!({"kind":"literal","value":{"fixed_input":id}});
        }
    }
    config["caches"]
        .as_object_mut()
        .unwrap()
        .retain(|_, cache| {
            cache["consumers"]
                .as_array_mut()
                .unwrap()
                .retain(|id| consumers.contains(&id.as_str().unwrap()));
            !cache["consumers"].as_array().unwrap().is_empty()
        });
    config["consumer_operations"]
        .as_object_mut()
        .unwrap()
        .retain(|id, _| consumers.contains(&id.as_str()));
    let mut files = std::collections::BTreeSet::from([
        ".chrono-harness/ci/check.json".to_string(),
        ".chrono-harness/ci/bootstrap-core.json".to_string(),
    ]);
    for refs in config["consumer_operations"].as_object().unwrap().values() {
        for source in refs.as_array().unwrap() {
            files.insert(source["registry"].as_str().unwrap().to_string());
        }
    }
    for path in files {
        fs::copy(host.join(&path), root.path().join(&path)).unwrap();
    }
    register_artifacts(root.path(), &config);
    fs::rename(
        root.path().join(".chrono-harness/artifacts.json"),
        root.path()
            .join(config["artifact_registry"].as_str().unwrap()),
    )
    .unwrap();
    (root, config)
}

#[test]
fn host_detector_cache_keys_ignore_check_policy_but_bind_core_bootstrap() {
    let (root, config) = host_cache_fixture(&["check.detect"]);
    let first = plan(root.path(), &config, "check.detect").unwrap();
    let checks = root.path().join(".chrono-harness/ci/check.json");
    let mut changed: Value = serde_json::from_slice(&fs::read(&checks).unwrap()).unwrap();
    changed["policy"]["units"]["locality-fixture"] = json!({"tests":["test:fixture"]});
    fs::write(&checks, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert_eq!(
        first["caches"],
        plan(root.path(), &config, "check.detect").unwrap()["caches"]
    );
    fs::remove_file(checks).unwrap();
    assert_eq!(
        first["caches"],
        plan(root.path(), &config, "check.detect").unwrap()["caches"]
    );
    let bootstrap = root.path().join(".chrono-harness/ci/bootstrap-core.json");
    let mut changed: Value = serde_json::from_slice(&fs::read(&bootstrap).unwrap()).unwrap();
    changed["rust_incremental"] = json!(!changed["rust_incremental"].as_bool().unwrap());
    fs::write(bootstrap, serde_json::to_vec(&changed).unwrap()).unwrap();
    let build_change = plan(root.path(), &config, "check.detect").unwrap();
    assert_ne!(
        first["caches"]["detect.ci"]["restore_keys"],
        build_change["caches"]["detect.ci"]["restore_keys"]
    );
}

#[test]
fn host_check_caches_ignore_unit_metadata_but_bind_build_environment_and_tool() {
    let consumers = ["check.unit_runner", "check.unit_ci"];
    let (root, config) = host_cache_fixture(&consumers);
    let checks = root.path().join(".chrono-harness/ci/check.json");
    let original: Value = serde_json::from_slice(&fs::read(&checks).unwrap()).unwrap();
    for consumer in consumers {
        fs::write(&checks, serde_json::to_vec(&original).unwrap()).unwrap();
        let first = plan(root.path(), &config, consumer).unwrap();
        let mut metadata = original.clone();
        metadata["policy"]["units"]["locality-fixture"] = json!({"tests":["test:fixture"]});
        fs::write(&checks, serde_json::to_vec(&metadata).unwrap()).unwrap();
        assert_eq!(
            first["caches"],
            plan(root.path(), &config, consumer).unwrap()["caches"]
        );
        for (pointer, value) in [
            ("/policy/environment/CARGO_INCREMENTAL", json!("0")),
            ("/policy/tools/cargo", json!("declared-other-cargo")),
        ] {
            let mut changed = original.clone();
            *changed.pointer_mut(pointer).unwrap() = value;
            fs::write(&checks, serde_json::to_vec(&changed).unwrap()).unwrap();
            let after = plan(root.path(), &config, consumer).unwrap();
            for (id, cache) in first["caches"].as_object().unwrap() {
                assert_ne!(
                    cache["restore_keys"], after["caches"][id]["restore_keys"],
                    "{consumer} {id} {pointer}"
                );
            }
        }
        fs::remove_file(&checks).unwrap();
        assert!(
            plan(root.path(), &config, consumer)
                .unwrap_err()
                .contains("check.json")
        );
    }
}

#[test]
fn json_value_inputs_bind_selected_semantics_and_retain_whole_file_provenance() {
    let (root, mut config) = fixture();
    config["inputs"]["profile"] =
        json!({"kind":"json-value","path":"settings.json","pointer":"/build"});
    let settings = root.path().join("settings.json");
    fs::write(
        &settings,
        br#"{"build":{"flags":["-g"],"incremental":true},"units":["one"]}"#,
    )
    .unwrap();
    let first = plan(root.path(), &config, "check.one").unwrap();
    fs::write(
        &settings,
        br#"{ "units": ["two"], "build": {"incremental":true,"flags":["-g"]} }"#,
    )
    .unwrap();
    let unrelated = plan(root.path(), &config, "check.one").unwrap();
    assert_eq!(first["caches"], unrelated["caches"]);
    assert_eq!(
        first["inputs"]["profile"]["observation"],
        unrelated["inputs"]["profile"]["observation"]
    );
    assert_ne!(
        first["inputs"]["profile"]["source_file"],
        unrelated["inputs"]["profile"]["source_file"]
    );
    fs::write(&settings, br#"{"build":null}"#).unwrap();
    assert_ne!(
        first["caches"],
        plan(root.path(), &config, "check.one").unwrap()["caches"]
    );
    fs::write(&settings, b"{}").unwrap();
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("missing JSON value settings.json#/build")
    );
    fs::write(&settings, b"invalid JSON").unwrap();
    assert!(plan(root.path(), &config, "check.one").is_err());
    fs::write(&settings, br#"{"a/b":{"~c":["selected"]}}"#).unwrap();
    config["inputs"]["profile"]["pointer"] = json!("/a~1b/~0c/0");
    assert!(plan(root.path(), &config, "check.one").is_ok());
    for pointer in ["build", "/a~2b", "/a~"] {
        config["inputs"]["profile"]["pointer"] = json!(pointer);
        assert!(
            plan(root.path(), &config, "check.one")
                .unwrap_err()
                .contains("RFC 6901")
        );
    }
    config["inputs"]["profile"]["pointer"] = json!("");
    assert!(plan(root.path(), &config, "check.one").is_ok());
    fs::create_dir_all(root.path().join("out/project")).unwrap();
    fs::copy(&settings, root.path().join("out/project/settings.json")).unwrap();
    config["inputs"]["profile"]["path"] = json!("out/project/settings.json");
    assert!(
        plan(root.path(), &config, "check.one")
            .unwrap_err()
            .contains("inside a selected cache artifact")
    );
}
