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
    struct ChainFixture {
        directory: tempfile::TempDir,
        case: &'static str,
    }
    impl Drop for ChainFixture {
        fn drop(&mut self) {
            if std::thread::panicking() {
                // Expected failures clean up normally. Keep unexpected failures
                // until diagnosis and acceptance consumers finish, then remove.
                self.directory.disable_cleanup(true);
                use std::io::Write;
                let root = self.directory.path();
                let _ = writeln!(
                    std::io::stderr(),
                    "Registered chain case {:?}: original fixture retained at {}\n\
                     Inputs: {} and {}\n\
                     Report: {}\n\
                     Process originals: {} (0=produce, 1=consume; available \
                     source-*.json, *.launch.json, *.stdout, *.stderr)\n\
                     Original report read: {:?}",
                    self.case,
                    root.display(),
                    root.join(".chrono-harness/projects.json").display(),
                    root.join(".chrono-harness/observe.json").display(),
                    root.join(".chrono-harness/state/chain.json").display(),
                    root.join(".chrono-harness/state/chain.json.processes")
                        .display(),
                    fs::read_to_string(root.join(".chrono-harness/state/chain.json"))
                );
            }
        }
    }
    for (first, expected_exit) in [("path", 0), ("fail", 17)] {
        let d = ChainFixture {
            directory: tempfile::tempdir().unwrap(),
            case: first,
        };
        let root = d.directory.path();
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
        let report_bytes = fs::read(root.join(output)).unwrap_or_else(|error| {
            panic!("chain case {first}: cannot read {output}: {error}; invocation: {result:?}")
        });
        let report: Value = serde_json::from_slice(&report_bytes).unwrap_or_else(|error| {
            panic!("chain case {first}: cannot decode {output}: {error}; invocation: {result:?}")
        });
        assert_eq!(
            report["operations"]["produce"]["exit_code"], expected_exit,
            "chain case {first}; invocation: {result:?}"
        );
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

// Expected behavior is independent of a native machine: the Linux spellings
// remain rejected by strict input validation and work only by explicit target
// acquisition, with both the selected spelling and actual aliases retained.
#[cfg(unix)]
#[test]
fn explicit_canonical_file_acquisition_preserves_linux_aliases_and_parent_components() {
    use std::os::unix::fs::symlink;
    let d = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(d.path()).unwrap();
    fs::create_dir_all(root.join("etc")).unwrap();
    fs::create_dir_all(root.join("usr/lib/gcc/x86_64-linux-gnu/13")).unwrap();
    fs::create_dir_all(root.join("usr/lib/x86_64-linux-gnu")).unwrap();
    fs::write(root.join("usr/lib/os-release"), b"native release").unwrap();
    fs::write(
        root.join("usr/lib/x86_64-linux-gnu/libc.so.6"),
        b"native libc",
    )
    .unwrap();
    symlink("../usr/lib/os-release", root.join("etc/os-release")).unwrap();
    symlink(
        "usr/lib/gcc/x86_64-linux-gnu/13",
        root.join("compiler-directory"),
    )
    .unwrap();
    for (index, spelling, actual, bytes) in [
        (
            0,
            "etc/os-release",
            "usr/lib/os-release",
            b"native release".as_slice(),
        ),
        (
            1,
            "usr/lib/gcc/x86_64-linux-gnu/13/../../../x86_64-linux-gnu/libc.so.6",
            "usr/lib/x86_64-linux-gnu/libc.so.6",
            b"native libc".as_slice(),
        ),
        (
            2,
            "compiler-directory/../../../x86_64-linux-gnu/libc.so.6",
            "usr/lib/x86_64-linux-gnu/libc.so.6",
            b"native libc".as_slice(),
        ),
    ] {
        let location = root.join(spelling);
        assert!(chrono_harness::input_file::observe_file(&location).is_err());
        let mut m = manifest(&root);
        m["directories"] = json!([]);
        m["operations"][0]["argv"] = json!(["path", location]);
        m["files"] = json!([{"id":"runtime","location":{"stdout":"location"}}]);
        let strict = format!(".chrono-harness/state/strict-{index}.json");
        assert!(invoke(&root, &m, &strict).is_err());
        m["files"][0]["resolution"] = json!("canonical-target");
        let output = format!(".chrono-harness/state/canonical-{index}.json");
        let report: Value = serde_json::from_str(&invoke(&root, &m, &output).unwrap()).unwrap();
        let file = &report["files"]["runtime"];
        assert_eq!(file["location"], json!(location));
        assert_eq!(file["resolution"]["location"], json!(root.join(actual)));
        assert_eq!(file["observation"]["sha256"], chrono_harness::sha256(bytes));
        let aliases = file["resolution"]["symlinks"].as_array().unwrap();
        if index == 0 {
            assert_eq!(
                aliases,
                &vec![json!({"path":root.join("etc/os-release"),
                "target":"../usr/lib/os-release","kind":"file"})]
            );
        } else if index == 2 {
            assert_eq!(
                aliases,
                &vec![json!({"path":root.join("compiler-directory"),
                "target":"usr/lib/gcc/x86_64-linux-gnu/13","kind":"directory"})]
            );
        } else {
            assert!(aliases.is_empty());
        }
        let process = chrono_harness::scoped_report::restore_process(
            &report["operations"]["location"],
            |o| chrono_harness::prepared::read_original(&root, o, None),
        )
        .unwrap();
        assert_eq!(
            process.stdout_bytes,
            format!("{}\n", location.display()).as_bytes()
        );
        assert!(chrono_harness::input_file::observe_file(&location).is_err());
    }
    symlink("missing", root.join("etc/dangling")).unwrap();
    let mut m = manifest(&root);
    m["operations"] = json!([]);
    m["directories"] = json!([]);
    m["files"] = json!([{"id":"dangling","location":{"path":"etc/dangling"},"resolution":"canonical-target"}]);
    let output = ".chrono-harness/state/dangling.json";
    assert!(invoke(&root, &m, output).is_err());
    let report: Value = serde_json::from_slice(&fs::read(root.join(output)).unwrap()).unwrap();
    assert!(report["files"].get("dangling").is_none());
    assert_eq!(report["status"], "failed");
}

#[test]
fn runtime_scope_observes_python_components_without_other_prefix_owners() {
    let d = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(d.path()).unwrap();
    for directory in ["usr/lib/python3.12/lib-dynload", "usr/lib/other-runtime"] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
    fs::write(root.join("usr/lib/python3.12/os.py"), b"stdlib").unwrap();
    fs::write(
        root.join("usr/lib/python3.12/lib-dynload/native.so"),
        b"extension",
    )
    .unwrap();
    fs::write(root.join("usr/lib/other-runtime/unrelated"), b"other owner").unwrap();
    let mut m = manifest(&root);
    m["operations"][0]["argv"] = json!(["path", root.join("usr/lib/python3.12")]);
    m["directories"][0]["max_entries"] = json!(3);
    let result: Value =
        serde_json::from_str(&invoke(&root, &m, ".chrono-harness/state/runtime.json").unwrap())
            .unwrap();
    let inventory = &result["directories"]["declared"];
    assert_eq!(inventory["root"], json!(root.join("usr/lib/python3.12")));
    assert_eq!(
        inventory["files"],
        json!([
        {"path":"lib-dynload/native.so","sha256":chrono_harness::sha256(b"extension")},
        {"path":"os.py","sha256":chrono_harness::sha256(b"stdlib")}
        ])
    );
}

#[test]
fn configuration_observation_retains_identity_and_absence_without_credential_contents() {
    let d = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(d.path()).unwrap();
    fs::create_dir(root.join("cargo")).unwrap();
    let secret = b"fixture credential content must stay at its owner";
    fs::write(root.join("cargo/credentials.toml"), secret).unwrap();
    let mut m = manifest(&root);
    m["operations"] = json!([]);
    m["directories"] = json!([]);
    m["files"] = json!([
        {"id":"present","location":{"path":"cargo/credentials.toml"}},
        {"id":"absent","location":{"path":"cargo/credentials"}}
    ]);
    let output = ".chrono-harness/state/credentials.json";
    let original = invoke(&root, &m, output).unwrap();
    let report: Value = serde_json::from_str(&original).unwrap();
    assert_eq!(
        report["files"]["present"]["observation"],
        json!({"sha256":chrono_harness::sha256(secret),"length":secret.len()})
    );
    assert_eq!(report["files"]["absent"]["absent"], true);
    assert!(!original.contains(std::str::from_utf8(secret).unwrap()));
    let retained: Vec<_> = fs::read_dir(root.join(format!("{output}.processes")))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(retained, vec![std::ffi::OsString::from("source-0.json")]);
    assert_eq!(
        fs::read(root.join("cargo/credentials.toml")).unwrap(),
        secret
    );
}

#[cfg(unix)]
#[test]
fn v3_manifest_is_published_from_real_nested_empty_and_alias_topology() {
    use std::os::unix::fs::symlink;
    let d = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(d.path()).unwrap();
    fs::create_dir_all(root.join("declared/nested/empty")).unwrap();
    fs::write(root.join("declared/nested/data"), b"directory bytes").unwrap();
    symlink("nested/data", root.join("declared/file-alias")).unwrap();
    symlink("file-alias", root.join("declared/chain")).unwrap();
    symlink("nested/empty", root.join("declared/directory-alias")).unwrap();
    let mut m = manifest(&root);
    let artifact = ".chrono-harness/state/guard/directory.json";
    m["directories"][0]["manifest"] = json!({"path":artifact,"root":"declared"});
    let output = ".chrono-harness/state/topology.json";
    let report: Value = serde_json::from_str(&invoke(&root, &m, output).unwrap()).unwrap();
    let reference =
        serde_json::from_value(report["directories"]["declared"]["manifest"].clone()).unwrap();
    let bytes = chrono_harness::prepared::read_original(&root, &reference, None).unwrap();
    let actual: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        actual,
        json!({"schema":"chrono-input-directory/v3","root":"declared",
        "files":[{"path":"nested/data","sha256":chrono_harness::sha256(b"directory bytes")}],
        "symlinks":[
            {"path":"chain","target":"file-alias","kind":"file"},
            {"path":"directory-alias","target":"nested/empty","kind":"directory"},
            {"path":"file-alias","target":"nested/data","kind":"file"}],
        "directories":[
            {"path":".","entries":["chain","directory-alias","file-alias","nested"]},
            {"path":"nested","entries":["data","empty"]},
            {"path":"nested/empty","entries":[]}]})
    );
    // Legacy observations retain their schema and literal untyped alias rows.
    assert_eq!(
        report["directories"]["declared"]["schema"],
        "chrono-input-directory-observation/v1"
    );
    assert!(
        report["directories"]["declared"]["symlinks"][0]
            .get("kind")
            .is_none()
    );
    fs::write(root.join("declared/new-child"), b"drift").unwrap();
    assert!(
        invoke(&root, &m, ".chrono-harness/state/reentry.json")
            .unwrap_err()
            .contains("EXISTS")
    );
    assert_eq!(fs::read(root.join(artifact)).unwrap(), bytes);
    assert!(
        !root
            .join(".chrono-harness/state/reentry.json.processes")
            .exists()
    );
}

#[cfg(unix)]
#[test]
fn rejected_manifest_aliases_retain_literal_resolution_and_kind_in_failed_originals() {
    use std::os::unix::fs::symlink;
    for case in [
        "absolute-inside",
        "relative-escape",
        "absolute-escape",
        "dangling",
        "loop",
        "unsupported-kind",
    ] {
        let d = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(d.path()).unwrap();
        fs::create_dir_all(root.join("declared/bin")).unwrap();
        fs::create_dir(root.join("declared/nested")).unwrap();
        fs::create_dir(root.join("other")).unwrap();
        fs::write(root.join("declared/data"), b"inside").unwrap();
        fs::write(root.join("other/data"), b"outside").unwrap();
        symlink("declared", root.join("selected")).unwrap();
        let socket = (case == "unsupported-kind").then(|| {
            std::os::unix::net::UnixListener::bind(root.join("declared/bin/z-socket")).unwrap()
        });
        let (literal, destination, kind) = match case {
            "absolute-inside" => (
                root.join("declared/nested/../data"),
                Some(root.join("declared/data")),
                Some("file"),
            ),
            "relative-escape" => (
                "../../other".into(),
                Some(root.join("other")),
                Some("directory"),
            ),
            "absolute-escape" => (
                root.join("other/data"),
                Some(root.join("other/data")),
                Some("file"),
            ),
            "dangling" => ("missing".into(), None, None),
            "loop" => ("alias".into(), None, None),
            // The alias sorts before the physical socket in the same directory.
            "unsupported-kind" => (
                "z-socket".into(),
                Some(root.join("declared/bin/z-socket")),
                Some("other"),
            ),
            _ => unreachable!(),
        };
        let alias = root.join("declared/bin/alias");
        symlink(&literal, &alias).unwrap();
        let resolution_error = if destination.is_none() {
            let error = fs::canonicalize(&alias).unwrap_err();
            Some(json!({"message":error.to_string(),"raw_os_error":error.raw_os_error()}))
        } else {
            None
        };
        let mut m = manifest(&root);
        m["operations"][0]["argv"][1] = json!(root.join("selected"));
        let artifact = ".chrono-harness/state/guard/directory.json";
        m["directories"][0]["manifest"] = json!({"path":artifact,"root":"selected"});
        let output = ".chrono-harness/state/alias-failed.json";
        assert!(
            invoke(&root, &m, output)
                .unwrap_err()
                .contains("E_OBSERVATION")
        );
        let original = fs::read(root.join(output)).unwrap();
        let report: Value = serde_json::from_slice(&original).unwrap();
        assert_eq!(report["status"], "failed", "{case}");
        assert_eq!(report["governance"], "not-evaluated");
        assert!(!root.join(artifact).exists(), "{case}");
        let directory = &report["directories"]["declared"];
        assert_eq!(directory["location"], json!(root.join("selected")));
        assert_eq!(directory["root"], json!(root.join("declared")));
        assert!(directory.get("manifest").is_none());
        assert!(directory.get("files").is_none());
        let failure = &directory["alias_failure"];
        assert_eq!(failure["path"], "bin/alias", "{case}");
        assert_eq!(failure["target"], json!(literal), "{case}");
        assert_eq!(failure["literal_absolute"], case.starts_with("absolute-"));
        assert_eq!(failure["canonical_destination"], json!(destination));
        assert_eq!(
            failure["within_root"],
            match case {
                "absolute-inside" | "unsupported-kind" => json!(true),
                "relative-escape" | "absolute-escape" => json!(false),
                _ => Value::Null,
            },
            "{case}"
        );
        assert_eq!(failure["resolution_error"], json!(resolution_error));
        assert_eq!(
            failure["target_kind"],
            match kind {
                Some(kind) => json!({"observed":kind}),
                None => json!({"unavailable":"canonical resolution failed"}),
            },
            "{case}"
        );
        let error = directory["error"].as_str().unwrap();
        if let Some(resolution_error) = resolution_error {
            assert_eq!(error, resolution_error["message"].as_str().unwrap());
        } else if case == "unsupported-kind" {
            assert_eq!(
                error,
                "directory manifest alias has unsupported target kind"
            );
        } else {
            assert_eq!(error, "directory manifest alias escapes scope: bin/alias");
        }
        assert_eq!(report["errors"], json!([format!("declared: {error}")]));
        let source = &report["sources"][0];
        assert_eq!(report["sources"].as_array().unwrap().len(), 1);
        assert_eq!(source["path"], ".chrono-harness/observe.json");
        let reference = serde_json::from_value(json!({
            "path":source["retained"],"sha256":source["sha256"]
        }))
        .unwrap();
        assert_eq!(
            chrono_harness::prepared::read_original(&root, &reference, None).unwrap(),
            serde_json::to_vec(&m).unwrap()
        );
        let process = chrono_harness::scoped_report::restore_process(
            &report["operations"]["location"],
            |o| chrono_harness::prepared::read_original(&root, o, None),
        )
        .unwrap();
        assert_eq!(process.exit_code, 0);
        assert_eq!(
            process.stdout_bytes,
            format!("{}\n", root.join("selected").display()).as_bytes()
        );
        assert!(process.stderr_bytes.is_empty());
        assert!(invoke(&root, &m, output).unwrap_err().contains("EXISTS"));
        assert_eq!(fs::read(root.join(output)).unwrap(), original);
        assert!(!root.join(artifact).exists());
        drop(socket);
    }
}

#[cfg(unix)]
#[test]
fn v3_manifest_failure_preserves_outputs_and_enforces_scope_aliases_and_entry_bounds() {
    use std::os::unix::fs::symlink;
    for case in ["bound", "escape", "dangling", "wrong-root", "success"] {
        let d = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(d.path()).unwrap();
        fs::create_dir_all(root.join("declared/empty")).unwrap();
        fs::create_dir(root.join("other")).unwrap();
        fs::write(root.join("declared/data"), b"bytes").unwrap();
        fs::write(root.join("other/data"), b"external").unwrap();
        let artifact = ".chrono-harness/state/guard/directory.json";
        let mut m = manifest(&root);
        m["directories"][0]["manifest"] = json!({"path":artifact,"root":"declared"});
        m["directories"][0]["max_entries"] = json!(2);
        match case {
            "bound" => m["directories"][0]["max_entries"] = json!(1),
            "wrong-root" => m["directories"][0]["manifest"]["root"] = json!("other"),
            "escape" | "dangling" => {
                m["directories"][0]["max_entries"] = json!(3);
                symlink(
                    if case == "escape" {
                        "../other/data"
                    } else {
                        "missing"
                    },
                    root.join("declared/alias"),
                )
                .unwrap();
            }
            _ => {}
        }
        let output = ".chrono-harness/state/manifest.json";
        let result = invoke(&root, &m, output);
        let original = fs::read(root.join(output)).unwrap();
        let report: Value = serde_json::from_slice(&original).unwrap();
        if case == "success" {
            assert!(result.is_ok(), "{result:?}");
            assert!(root.join(artifact).is_file());
        } else {
            assert!(result.is_err(), "{case}");
            assert_eq!(report["status"], "failed");
            assert!(!root.join(artifact).exists());
            assert!(report["operations"]["location"]["stdout_original"].is_object());
            assert!(!report["errors"].as_array().unwrap().is_empty());
        }
        assert!(invoke(&root, &m, output).unwrap_err().contains("EXISTS"));
        assert_eq!(fs::read(root.join(output)).unwrap(), original);
    }
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
    for native in [&manifest, &linux] {
        for id in [
            "configuration.0a24bcbbe88ebae0",
            "configuration.22e1529f27755c5a",
            "configuration.4971e26cb7d81352",
            "configuration.53f66258050caac2",
            "configuration.6578aa671e061376",
            "configuration.70d663ddab3591b1",
            "configuration.70f054831d3eb9be",
            "configuration.770c0f688c0d8fc5",
            "configuration.98f18b28b308b630",
            "configuration.9cef6b2441aab2d0",
            "configuration.9f2e73c2801230e0",
            "configuration.b8e1dc7fc228914e",
            "configuration.c7dc5e26eaf9702a",
            "configuration.cd80172321df8988",
            "configuration.credentials",
            "configuration.credentials.toml",
            "configuration.e3e19abc5bd066d6",
            "configuration.eca50d195b820816",
            "configuration.rustup-settings",
            "runtime.python.shared-library",
        ] {
            assert_eq!(
                native["files"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|r| r["id"] == id)
                    .count(),
                1,
                "{id}"
            );
        }
        for id in ["tool.native-cc", "tool.rustup", "tool.cargo-fmt"] {
            let op = native["operations"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == id)
                .unwrap();
            assert_eq!(op["argv"], json!(["--version"]));
        }
        for id in ["python", "python.platstdlib", "python.extensions"] {
            let dir = native["directories"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["id"] == id)
                .unwrap();
            assert_eq!(dir["max_entries"], 200000);
        }
    }
    let guard: Value = serde_json::from_slice(
        &fs::read(root.join(".chrono-harness/native/macos/cargo-runner.json")).unwrap(),
    )
    .unwrap();
    let config: Value = serde_json::from_slice(
        &fs::read(root.join(".chrono-harness/native/macos/config.json")).unwrap(),
    )
    .unwrap();
    for (index, id) in [(0, "sdk"), (1, "developer")] {
        let declaration = manifest["directories"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == id)
            .unwrap();
        let scope = &guard["toolchain"]["sdks"][index];
        let input = config["environment"]["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == scope["manifest"])
            .unwrap();
        assert_eq!(declaration["manifest"]["root"], scope["root"]);
        assert_eq!(declaration["manifest"]["path"], input["location"]);
        assert_eq!(declaration["max_entries"], 200000);
    }
}
