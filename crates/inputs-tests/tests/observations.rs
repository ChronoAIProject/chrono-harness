use serde_json::{Value, json};
use std::{fs, path::Path};

fn invoke(root: &Path, manifest: &Value, output: &str) -> Result<String, String> {
    fs::create_dir_all(root.join(".chrono-harness")).unwrap();
    fs::write(
        root.join(".chrono-harness/observe.json"),
        serde_json::to_vec(manifest).unwrap(),
    )
    .unwrap();
    let stdout = chrono_inputs::run(
        root,
        &[
            "observe",
            "--host-root",
            ".",
            "--manifest",
            ".chrono-harness/observe.json",
            "--output",
            output,
        ]
        .map(String::from)
        .to_vec(),
    )?;
    let reference: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(reference["schema"], "chrono-input-observation-reference/v1");
    assert!(stdout.len() < 1024);
    let original = serde_json::from_value(reference["original"].clone()).unwrap();
    String::from_utf8(chrono_harness::prepared::read_original(
        root, &original, None,
    )?)
    .map_err(|e| e.to_string())
}
fn manifest(root: &Path) -> Value {
    json!({"schema":"chrono-input-observation/v1", "environment":{"inherit":[], "values":{}},
        "operations":[{"id":"location", "program":env!("CARGO_BIN_EXE_chrono-inputs-test-observe"),
            "argv":["path",root.join("declared").to_str().unwrap()], "timeout_seconds":5,"output_limit_bytes":4096}],
        "files":[{"id":"missing","location":{"path":"absent"}}],
        "directories":[{"id":"declared","location":{"stdout":"location"},"max_entries":16}]})
}
#[test]
fn observations_retain_actual_processes_absence_and_only_declared_roots() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    fs::create_dir(root.join("declared")).unwrap();
    fs::write(root.join("declared/data"), b"actual bytes").unwrap();
    fs::write(root.join("unrelated"), b"not selected").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("data", root.join("declared/alias")).unwrap();
    let m = manifest(root);
    let output = ".chrono-harness/state/observation.json";
    let result: Value = serde_json::from_str(&invoke(root, &m, output).unwrap()).unwrap();
    assert_eq!(result["status"], "observed");
    assert_eq!(result["governance"], "not-evaluated");
    assert_eq!(result["files"]["missing"]["absent"], true);
    let inv = &result["directories"]["declared"];
    assert_eq!(inv["files"][0]["path"], "data");
    assert_eq!(
        inv["files"][0]["sha256"],
        chrono_harness::sha256(b"actual bytes")
    );
    assert_eq!(inv["files"].as_array().unwrap().len(), 1);
    #[cfg(unix)]
    assert_eq!(inv["symlinks"][0], json!({"path":"alias","target":"data"}));
    let process = &result["operations"]["location"];
    assert_eq!(process["exit_code"], 0);
    assert_eq!(
        process["sha256"],
        chrono_harness::file_identity(Path::new(env!("CARGO_BIN_EXE_chrono-inputs-test-observe")))
            .unwrap()
            .0
    );
    assert!(process["environment"].as_object().unwrap().is_empty());
    assert!(process.get("stdout_bytes").is_none());
    let restore = || {
        chrono_harness::scoped_report::restore_process(process, |o| {
            chrono_harness::prepared::read_original(root, o, None)
        })
    };
    let original_process = restore().unwrap();
    assert_eq!(
        original_process.stdout_bytes,
        format!("{}\n", root.join("declared").display()).as_bytes()
    );
    let stream = root.join(process["stdout_original"]["path"].as_str().unwrap());
    fs::write(&stream, b"tampered").unwrap();
    assert!(restore().is_err());
    fs::remove_file(&stream).unwrap();
    assert!(restore().is_err());
    fs::write(&stream, &original_process.stdout_bytes).unwrap();
    assert!(root.join(format!("{output}.processes/0.stdout")).is_file());
    assert!(
        root.join(format!("{output}.processes/0.launch.json"))
            .is_file()
    );
    let original = fs::read(root.join(output)).unwrap();
    fs::write(root.join("declared/data"), b"changed").unwrap();
    assert!(invoke(root, &m, output).unwrap_err().contains("EXISTS"));
    assert_eq!(fs::read(root.join(output)).unwrap(), original);
}

#[test]
fn native_observation_executes_selected_tool_and_rejects_missing_failed_or_wrong_platform_selection()
 {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    let mut m = manifest(root);
    m["directories"] = json!([]);
    m["platforms"] = json!([format!(
        "{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )]);
    m["operations"][0]["id"] = json!("selection");
    m["operations"][0]["argv"] = json!(["path", env!("CARGO_BIN_EXE_chrono-inputs-test-observe")]);
    m["operations"].as_array_mut().unwrap().push(
        json!({"id":"selected", "program":{"stdout":"selection"},
        "environment":{"RUSTC":{"stdout":"selection"}},
        "argv":["path",root],"timeout_seconds":5,"output_limit_bytes":4096}),
    );
    let result: Value =
        serde_json::from_str(&invoke(root, &m, ".chrono-harness/state/selected.json").unwrap())
            .unwrap();
    let actual =
        chrono_harness::scoped_report::restore_process(&result["operations"]["selected"], |o| {
            chrono_harness::prepared::read_original(root, o, None)
        })
        .unwrap();
    assert_eq!(
        actual.executable,
        Path::new(env!("CARGO_BIN_EXE_chrono-inputs-test-observe"))
    );
    assert_eq!(
        actual.environment["RUSTC"],
        env!("CARGO_BIN_EXE_chrono-inputs-test-observe")
    );
    assert_eq!(
        actual.stdout_bytes,
        format!("{}\n", root.display()).as_bytes()
    );
    m["platforms"] = json!(["unsupported-native"]);
    assert!(
        invoke(root, &m, ".chrono-harness/state/platform.json")
            .unwrap_err()
            .contains("native platform")
    );
    assert!(
        !root
            .join(".chrono-harness/state/platform.json.processes")
            .exists()
    );
    m.as_object_mut().unwrap().remove("platforms");
    m["operations"][1]["program"] = json!({"stdout":"missing"});
    assert!(
        invoke(root, &m, ".chrono-harness/state/missing.json")
            .unwrap_err()
            .contains("preceding")
    );
    m["operations"][1]["program"] = json!({"stdout":"selection"});
    m["operations"][0]["argv"] = json!(["fail"]);
    assert!(invoke(root, &m, ".chrono-harness/state/selection-failed.json").is_err());
    let failed: Value = serde_json::from_slice(
        &fs::read(root.join(".chrono-harness/state/selection-failed.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(failed["operations"]["selection"]["exit_code"], 17);
    assert!(failed["operations"].get("selected").is_none());
    assert!(
        !root
            .join(".chrono-harness/state/selection-failed.json.processes/1.launch.json")
            .exists()
    );
    m["operations"][0]["argv"] = json!(["path", "two\npaths"]);
    assert!(invoke(root, &m, ".chrono-harness/state/ambiguous-program.json").is_err());
}
#[test]
fn stdout_file_locations_preserve_absent_and_explicit_suffix_paths() {
    for suffix in [None, Some(""), Some("file")] {
        let d = tempfile::tempdir().unwrap();
        let canonical = fs::canonicalize(d.path()).unwrap();
        let root = canonical.as_path();
        let file = root.join("file");
        fs::write(&file, b"observed file bytes").unwrap();
        let mut m = manifest(root);
        m["directories"] = json!([]);
        m["operations"][0]["argv"] = if suffix == Some("file") {
            json!(["path", root])
        } else {
            json!(["path", file])
        };
        m["files"] = json!([{"id":"file", "location":{"stdout":"location"}}]);
        if let Some(suffix) = suffix {
            m["files"][0]["location"]["suffix"] = json!(suffix);
        }
        let report: Value = serde_json::from_str(
            &invoke(root, &m, ".chrono-harness/state/file.json").unwrap_or_else(|error| {
                panic!(
                    "{error}: {}",
                    fs::read_to_string(root.join(".chrono-harness/state/file.json"))
                        .unwrap_or_default()
                )
            }),
        )
        .unwrap();
        assert_eq!(report["files"]["file"]["location"], json!(file));
        assert_eq!(report["files"]["file"]["observation"]["length"], 19);
        assert_eq!(
            report["files"]["file"]["observation"]["sha256"],
            chrono_harness::sha256(b"observed file bytes")
        );
        fs::remove_file(&file).unwrap();
        let report: Value = serde_json::from_str(
            &invoke(root, &m, ".chrono-harness/state/absent-file.json").unwrap(),
        )
        .unwrap();
        assert_eq!(report["files"]["file"]["absent"], true);
    }
}

#[test]
fn failed_bounded_children_and_invalid_location_keep_original_evidence() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    let mut m = manifest(root);
    m["operations"][0]["argv"] = json!(["fail"]);
    let output = ".chrono-harness/state/failed.json";
    assert!(invoke(root, &m, output).is_err());
    let result: Value = serde_json::from_slice(&fs::read(root.join(output)).unwrap()).unwrap();
    assert_eq!(result["status"], "failed");
    assert_eq!(result["operations"]["location"]["exit_code"], 17);
    assert_eq!(
        fs::read(root.join(format!("{output}.processes/0.stderr"))).unwrap(),
        b"actual failure\n"
    );
    m["operations"][0]["argv"] = json!(["overflow"]);
    m["operations"][0]["output_limit_bytes"] = json!(32);
    assert!(invoke(root, &m, ".chrono-harness/state/overflow.json").is_err());
    let raw =
        fs::read(root.join(".chrono-harness/state/overflow.json.processes/0.stdout")).unwrap();
    assert!(raw.len() <= 32);
    m["directories"][0]["location"] = json!({"stdout":"unregistered"});
    assert!(invoke(root, &m, ".chrono-harness/state/invalid.json").is_err());
    assert!(
        !root
            .join(".chrono-harness/state/invalid.json.processes")
            .exists()
    );
}

#[test]
fn registered_operations_reuse_owner_argv_and_retain_failed_or_ambiguous_bindings() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    fs::create_dir(root.join("declared")).unwrap();
    let mut m = manifest(root);
    m["operations"] = json!([]);
    m["registered_operations"] = json!({"projects":".chrono-harness/projects.json","operations":["observe.path"],"tools":{"probe":env!("CARGO_BIN_EXE_chrono-inputs-test-observe")},"timeout_seconds":5,"output_limit_bytes":4096});
    m["directories"][0]["location"] = json!({"stdout":"observe.path"});
    fs::create_dir_all(root.join(".chrono-harness")).unwrap();
    let mut projects = json!({"projects":[{"id":"real","actions":{"probe":{"operation":"observe.path","tool":"probe","argv":["path",root.join("declared").to_str().unwrap()]}}}]});
    fs::write(
        root.join(".chrono-harness/projects.json"),
        serde_json::to_vec(&projects).unwrap(),
    )
    .unwrap();
    let value: Value =
        serde_json::from_str(&invoke(root, &m, ".chrono-harness/state/registered.json").unwrap())
            .unwrap();
    assert_eq!(value["operations"]["observe.path"]["argv"][1], "path");
    assert_eq!(
        value["directories"]["declared"]["root"],
        fs::canonicalize(root.join("declared"))
            .unwrap()
            .to_str()
            .unwrap()
    );
    // Registered Python/shell acquisition lives at script owners too. The same
    // operation cannot be ambiguously declared by a script and a project.
    projects["scripts"] = projects["projects"].clone();
    projects["projects"] = json!([]);
    fs::write(
        root.join(".chrono-harness/projects.json"),
        serde_json::to_vec(&projects).unwrap(),
    )
    .unwrap();
    let script: Value =
        serde_json::from_str(&invoke(root, &m, ".chrono-harness/state/script.json").unwrap())
            .unwrap();
    assert_eq!(script["operations"]["observe.path"]["exit_code"], 0);
    projects["projects"] = projects["scripts"].clone();
    fs::write(
        root.join(".chrono-harness/projects.json"),
        serde_json::to_vec(&projects).unwrap(),
    )
    .unwrap();
    assert!(
        invoke(root, &m, ".chrono-harness/state/ambiguous.json")
            .unwrap_err()
            .contains("ambiguous")
    );
    assert!(
        !root
            .join(".chrono-harness/state/ambiguous.json.processes")
            .exists()
    );
}

#[test]
fn registered_chain_preserves_distinct_bounds_and_stops_after_failure() {
    for (first, expected_exit) in [("path", 0), ("fail", 17)] {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        fs::create_dir_all(root.join(".chrono-harness")).unwrap();
        fs::write(
            root.join(".chrono-harness/projects.json"),
            serde_json::to_vec(&json!({
                "projects":[],"scripts":[{"actions":{
                    "produce":{"operation":"produce","tool":"probe","argv":[first, "produced"]},
                    "consume":{"operation":"consume","tool":"probe","argv":["overflow"]}
                }}]
            }))
            .unwrap(),
        )
        .unwrap();
        let m = json!({
            "schema":"chrono-input-observation/v1",
            "environment":{"inherit":[],"values":{}},
            "operations":[],"files":[],"directories":[],"stop_on_failure":true,
            "registered_operations":{"projects":".chrono-harness/projects.json",
                "operations":["produce","consume"],
                "tools":{"probe":env!("CARGO_BIN_EXE_chrono-inputs-test-observe")},
                "timeout_seconds":2,"output_limit_bytes":32,
                "operation_bounds":{"consume":{"timeout_seconds":3,"output_limit_bytes":8192}}}
        });
        let output = ".chrono-harness/state/chain.json";
        let result = invoke(root, &m, output);
        let report: Value = serde_json::from_slice(&fs::read(root.join(output)).unwrap()).unwrap();
        assert_eq!(report["operations"]["produce"]["exit_code"], expected_exit);
        if expected_exit == 0 {
            assert!(result.is_ok(), "{result:?}");
            assert_eq!(report["operations"]["consume"]["exit_code"], 0);
            assert_eq!(
                fs::read(root.join(format!("{output}.processes/1.stdout")))
                    .unwrap()
                    .len(),
                4096
            );
        } else {
            assert!(result.is_err());
            assert!(report["operations"].get("consume").is_none());
            assert!(
                !root
                    .join(format!("{output}.processes/1.launch.json"))
                    .exists()
            );
            assert_eq!(
                fs::read(root.join(format!("{output}.processes/0.stderr"))).unwrap(),
                b"actual failure\n"
            );
        }
    }
}

#[test]
fn registered_chain_rejects_unknown_or_zero_bounds_before_effects() {
    for bounds in [
        json!({"unknown":{"timeout_seconds":2,"output_limit_bytes":32}}),
        json!({"produce":{"timeout_seconds":0,"output_limit_bytes":32}}),
    ] {
        let d = tempfile::tempdir().unwrap();
        let root = d.path();
        fs::create_dir_all(root.join(".chrono-harness")).unwrap();
        fs::write(
            root.join(".chrono-harness/projects.json"),
            serde_json::to_vec(&json!({
                "projects":[],"scripts":[{"actions":{"produce":{
                    "operation":"produce","tool":"probe","argv":["path","produced"]}}}]
            }))
            .unwrap(),
        )
        .unwrap();
        let m = json!({"schema":"chrono-input-observation/v1",
            "environment":{"inherit":[],"values":{}},"operations":[],"files":[],"directories":[],
            "registered_operations":{"projects":".chrono-harness/projects.json","operations":["produce"],
                "tools":{"probe":env!("CARGO_BIN_EXE_chrono-inputs-test-observe")},
                "timeout_seconds":2,"output_limit_bytes":32,"operation_bounds":bounds}});
        assert!(invoke(root, &m, ".chrono-harness/state/invalid-chain.json").is_err());
        assert!(
            !root
                .join(".chrono-harness/state/invalid-chain.json.processes")
                .exists()
        );
    }
}

#[test]
fn inventory_bounds_and_child_timeout_remain_failures_with_originals() {
    let d = tempfile::tempdir().unwrap();
    let root = d.path();
    fs::create_dir(root.join("declared")).unwrap();
    for name in ["one", "two"] {
        fs::write(root.join("declared").join(name), name).unwrap();
    }
    let mut m = manifest(root);
    m["directories"][0]["max_entries"] = json!(1);
    assert!(invoke(root, &m, ".chrono-harness/state/bound.json").is_err());
    let r: Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/state/bound.json")).unwrap())
            .unwrap();
    assert!(r["errors"][0].as_str().unwrap().contains("bound"));
    m["operations"][0]["argv"] = json!(["timeout"]);
    m["operations"][0]["timeout_seconds"] = json!(1);
    assert!(invoke(root, &m, ".chrono-harness/state/timeout.json").is_err());
    let r: Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/state/timeout.json")).unwrap())
            .unwrap();
    assert_eq!(r["operations"]["location"]["failure"], "process timed out");
    assert_eq!(
        fs::read(root.join(".chrono-harness/state/timeout.json.processes/0.stderr")).unwrap(),
        b"before timeout\n"
    );
}

#[test]
fn main_host_observation_reuses_all_declared_cargo_acquisition_operations() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest: Value = serde_json::from_slice(
        &fs::read(root.join(".chrono-harness/inputs/native-observation.json")).unwrap(),
    )
    .unwrap();
    let owner: Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/cargo/observe.json")).unwrap())
            .unwrap();
    let expected: Vec<String> = owner["projects"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|p| {
            [
                format!("inputs.fetch.{}", p.as_str().unwrap()),
                format!("inputs.metadata.{}", p.as_str().unwrap()),
            ]
        })
        .collect();
    assert_eq!(
        manifest["registered_operations"]["operations"],
        json!(expected)
    );
    let projects: Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/projects.json")).unwrap())
            .unwrap();
    for op in expected {
        let actions: Vec<&Value> = projects["projects"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|p| p["actions"].as_object().unwrap().values())
            .filter(|a| a["operation"] == op)
            .collect();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0]["tool"], "cargo");
    }
    assert_eq!(
        manifest["environment"]["values"]["RUSTUP_TOOLCHAIN"],
        "1.95.0"
    );
    assert!(
        !manifest["environment"]["inherit"]
            .as_array()
            .unwrap()
            .contains(&json!("GITHUB_JOB"))
    );
    let linux: Value = serde_json::from_slice(
        &fs::read(root.join(".chrono-harness/inputs/linux-observation.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(linux["platforms"], json!(["linux-x86_64"]));
    for op in linux["registered_operations"]["operations"]
        .as_array()
        .unwrap()
    {
        let actions: Vec<_> = projects["projects"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|p| p["actions"].as_object().unwrap().values())
            .filter(|a| a["operation"] == *op)
            .collect();
        assert_eq!(actions.len(), 1);
        if op.as_str().unwrap().starts_with("inputs.metadata.") {
            assert!(
                actions[0]["argv"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("x86_64-unknown-linux-gnu"))
            );
            assert!(
                !actions[0]["argv"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("aarch64-apple-darwin"))
            );
        }
    }
    let provisioning: Value = serde_json::from_slice(
        &fs::read(root.join(".chrono-harness/inputs/native-provisioning.json")).unwrap(),
    )
    .unwrap();
    let fetch: Vec<_> = manifest["registered_operations"]["operations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|id| id.as_str().unwrap().starts_with("inputs.fetch."))
        .cloned()
        .collect();
    assert_eq!(
        provisioning["registered_operations"]["operations"],
        json!(fetch)
    );
    assert_eq!(
        provisioning["registered_operations"]["tools"]["cargo"],
        json!({"stdout":"selection.cargo"})
    );
    assert_eq!(
        provisioning["registered_operations"]["environment"]["RUSTC"],
        json!({"stdout":"selection.rustc"})
    );
}
