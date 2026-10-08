use serde_json::{Value, json};
use std::{fs, path::Path};

fn invoke(root: &Path, manifest: &Value, output: &str) -> Result<String, String> {
    fs::create_dir_all(root.join(".chrono-harness")).unwrap();
    fs::write(
        root.join(".chrono-harness/observe.json"),
        serde_json::to_vec(manifest).unwrap(),
    )
    .unwrap();
    chrono_inputs::run(
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
    )
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
    let duplicate = projects["projects"][0].clone();
    projects["projects"].as_array_mut().unwrap().push(duplicate);
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
}
