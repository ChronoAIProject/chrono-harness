use chrono_cache::{Config, prepare};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn fixture() -> (tempfile::TempDir, Value) {
    let root = tempfile::tempdir().unwrap();
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
    prepare(root, &config, consumer)
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
