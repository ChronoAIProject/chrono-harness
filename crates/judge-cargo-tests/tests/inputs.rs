#[path = "support/cargo.rs"]
mod cargo;
use cargo::*;
use chrono_harness::sha256;
use serde_json::json;
use std::fs;
use std::process::Command;

#[test]
fn pinned_git_package_executes_and_changed_revision_blocks_the_consumer() {
    let mut f = Fixture::new();
    let root = f.root();
    let upstream = tempfile::tempdir().unwrap();
    write(
        upstream.path(),
        "Cargo.toml",
        "[package]\nname='fixture-git'\nversion='1.0.0'\nedition='2021'\n",
    );
    write(upstream.path(), "src/lib.rs", "pub fn value()->u32 { 0 }\n");
    git(upstream.path(), &["init", "-q"]);
    let revision = commit(upstream.path());
    let url = format!(
        "file://{}",
        fs::canonicalize(upstream.path()).unwrap().display()
    );
    let manifest = f.root().join("p/Cargo.toml");
    let mut text = fs::read_to_string(&manifest).unwrap();
    text.push_str(&format!(
        "git_value={{package='fixture-git',git='{url}',rev='{revision}'}}\n"
    ));
    fs::write(manifest, text).unwrap();
    write(
        &f.root(),
        "p/src/lib.rs",
        "pub fn value()->u32 { value_dep::value()+1+git_value::value() }\n",
    );
    let cargo = |args: &[&str]| {
        let mut cmd = Command::new(f.values[CONFIG]["tools"][0]["program"].as_str().unwrap());
        cmd.current_dir(f.root())
            .env_clear()
            .env("PATH", std::env::var("PATH").unwrap());
        for (key, value) in f.values[CONFIG]["environment"]["values"]
            .as_object()
            .unwrap()
        {
            cmd.env(key, value.as_str().unwrap());
        }
        let out = cmd.args(args).output().unwrap();
        assert!(
            out.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    };
    let config = ".chrono-harness/cargo/config.toml";
    for manifest in ["p/Cargo.toml", "t/Cargo.toml"] {
        // Only the local file:// repository is fetched during fixture setup.
        cargo(&[
            "generate-lockfile",
            "--manifest-path",
            manifest,
            "--config",
            config,
        ]);
    }
    let destination = f.root().join(".chrono-harness/state/git-vendor");
    let vendored = cargo(&[
        "vendor",
        "--locked",
        "--offline",
        "--respect-source-config",
        "--manifest-path",
        "t/Cargo.toml",
        "--config",
        config,
        destination.to_str().unwrap(),
    ]);
    fs::write(f.root().join(config), vendored.stdout).unwrap();
    for row in f.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
    {
        let path = row["location"]
            .as_str()
            .unwrap()
            .replace("/vendor/", "/git-vendor/");
        row["sha256"] = sha256(&fs::read(root.join(&path)).unwrap()).into();
        row["location"] = path.into();
    }
    let mut inputs = Vec::new();
    for (suffix, path) in [
        ("manifest", "Cargo.toml"),
        ("lib", "src/lib.rs"),
        ("checksums", ".cargo-checksum.json"),
    ] {
        let id = format!("git.{suffix}");
        let location = format!(".chrono-harness/state/git-vendor/fixture-git/{path}");
        f.values.get_mut(CONFIG).unwrap()["environment"]["inputs"].as_array_mut().unwrap().push(json!({"id":id,"location":location,"sha256":sha256(&fs::read(root.join(&location)).unwrap())}));
        f.values.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap()
            .push(edge(&format!("input:{id}"), "runtime-input", "project:p"));
        inputs.push(id);
    }
    f.contract["packages"][0]["dependencies"]
        .as_array_mut()
        .unwrap()
        .push(json!({"package":"git","name":"git_value","kinds":[{"kind":null,"target":null}]}));
    f.contract["packages"].as_array_mut().unwrap().push(json!({"id":"git","name":"fixture-git","version":"1.0.0","source":format!("git+{url}?rev={revision}#{revision}"),"project":null,"manifest_input":"git.manifest","inputs":inputs,"checksum":null,"features":[],"dependencies":[]}));
    f.save();
    let (code, report, stderr) = f.call();
    assert_eq!(code, 0, "{report} {stderr}");
    assert_eq!(report["metadata"]["exit_code"], 0);
    assert_eq!(report["operation"]["exit_code"], 0);
    assert_eq!(
        fs::read(f.root().join(".chrono-harness/state/test-ran")).unwrap(),
        b"ran"
    );
    fs::remove_file(f.root().join(".chrono-harness/state/test-ran")).unwrap();
    f.contract["packages"][4]["source"] =
        format!("git+{url}?rev={revision}#{}", "0".repeat(40)).into();
    f.save();
    let (code, report, stderr) = f.call();
    assert_ne!(code, 0, "{report} {stderr}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("lock identity/checksum")
    );
    assert!(report["metadata"].is_null() && report["operation"].is_null());
    assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
}

#[test]
fn guarded_registry_and_transitive_inputs_execute_real_cargo_test() {
    let f = Fixture::new();
    let (code, report, stderr) = f.call();
    assert_eq!(
        code, 0,
        "declared registry input guard must run: {report} {stderr}"
    );
    assert_eq!(report["metadata"]["exit_code"], 0);
    assert!(
        report["metadata"]["stdout"]
            .as_str()
            .unwrap()
            .contains("fixture-leaf")
    );
    assert_eq!(report["operation"]["exit_code"], 0);
    assert!(
        report["operation"]["stdout"]
            .as_str()
            .unwrap()
            .contains("1 passed")
    );
    assert_eq!(
        fs::read(f.root().join(".chrono-harness/state/test-ran")).unwrap(),
        b"ran"
    );
}

#[test]
fn registered_named_suites_filters_and_libtest_tails_preserve_real_listings_and_assertions() {
    let mut f = Fixture::new();
    write(
        &f.root(),
        "t/tests/groups.rs",
        "#[test] fn core_pass() { assert_eq!(p::value(),43); }\n#[test] fn detail_pass() { assert_eq!(p::value(),43); }\n#[test] fn selected_failure() { panic!(\"real selected assertion\"); }\n",
    );
    let mut row = file(
        "t/tests/groups.rs",
        json!([{"kind":"compile","to":"project:t"}]),
    );
    row["owner"] = "t".into();
    f.values.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(row);
    let original = f.contract["operations"]["test.t"]["argv"]
        .as_array()
        .unwrap()
        .clone();
    for (tail, expected, listing) in [
        (
            vec!["--test", "groups", "--", "--list"],
            0,
            "core_pass: test",
        ),
        (
            vec![
                "--test",
                "groups",
                "detail",
                "--",
                "--list",
                "--exact",
                "detail_pass",
            ],
            0,
            "detail_pass: test",
        ),
        (
            vec![
                "--test",
                "groups",
                "--",
                "--skip",
                "selected_failure",
                "--test-threads=1",
            ],
            0,
            "2 passed",
        ),
        (
            vec![
                "--test",
                "groups",
                "selected_failure",
                "--",
                "--exact",
                "--nocapture",
            ],
            101,
            "selected_failure",
        ),
    ] {
        let mut argv = original.clone();
        argv.extend(tail.iter().map(|s| json!(s)));
        f.contract["operations"]["test.t"]["argv"] = json!(argv);
        f.save();
        let (code, report, stderr) = f.call();
        assert_eq!(code, expected, "{report} {stderr}");
        let process = &report["operation"];
        assert_eq!(process["exit_code"], expected, "{report}");
        assert!(
            process["stdout"].as_str().unwrap().contains(listing),
            "{report}"
        );
        let bytes: Vec<u8> = serde_json::from_value(process["stdout_bytes"].clone()).unwrap();
        assert_eq!(process["stdout_sha256"], sha256(&bytes));
    }
}

#[test]
fn native_v6_keeps_the_original_output_layout_and_checks_real_host_and_offline_selection() {
    for case in [
        "native",
        "wrong-host",
        "target-env",
        "online",
        "old-schema",
        "format",
    ] {
        let mut f = Fixture::new();
        toolchain_binding(&mut f);
        f.contract["schema"] = json!("chrono-cargo-inputs/v6");
        f.contract["target_mode"] = json!("native");
        f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CARGO_NET_OFFLINE"] =
            json!("true");
        let argv = f.contract["operations"]["test.t"]["argv"]
            .as_array_mut()
            .unwrap();
        let index = argv.iter().position(|v| v == "--target").unwrap();
        argv.drain(index..index + 2);
        argv.retain(|v| v != "--offline");
        match case {
            "wrong-host" => {
                f.contract["target"] = json!("wrong-native-host");
                let argv = f.contract["metadata"]["argv"].as_array_mut().unwrap();
                let index = argv.iter().position(|v| v == "--filter-platform").unwrap();
                argv[index + 1] = json!("wrong-native-host");
            }
            "target-env" => {
                f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CARGO_BUILD_TARGET"] =
                    f.contract["target"].clone()
            }
            "online" => f.values.get_mut(CONFIG).unwrap()["environment"]["values"]
                .as_object_mut()
                .unwrap()
                .remove("CARGO_NET_OFFLINE")
                .map(|_| ())
                .unwrap(),
            "old-schema" => f.contract["schema"] = json!("chrono-cargo-inputs/v5"),
            "format" => f.contract["operations"]["test.t"]["argv"][0] = json!("fmt"),
            _ => {}
        }
        f.save();
        let (code, report, stderr) = f.call();
        if case == "native" {
            assert_eq!(code, 0, "{report} {stderr}");
            assert!(
                report["native_target"]["stdout"]
                    .as_str()
                    .unwrap()
                    .contains("host: ")
            );
            assert!(
                report["operation"]["stdout"]
                    .as_str()
                    .unwrap()
                    .contains("1 passed")
            );
            assert!(f.root().join("t/target/debug/deps").is_dir());
            assert!(
                !f.root()
                    .join("t/target")
                    .join(f.contract["target"].as_str().unwrap())
                    .exists()
            );
        } else {
            assert_ne!(code, 0, "{case}: {report} {stderr}");
            assert!(report["operation"].is_null());
            assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
            if case == "wrong-host" {
                assert!(report["native_target"].is_object(), "{report}");
                assert!(
                    report["error"]
                        .as_str()
                        .unwrap()
                        .contains("native host differs"),
                    "{report}"
                );
            }
        }
    }
}

#[test]
fn main_host_declared_metadata_operations_retain_actual_resolution_streams() {
    use chrono_harness::{ProcessEvidence, observation, run_process_observed_retained};
    use std::{
        collections::BTreeMap,
        time::{SystemTime, UNIX_EPOCH},
    };
    let started = std::time::Instant::now();
    let root = fs::canonicalize(source()).unwrap();
    let projects: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/projects.json")).unwrap())
            .unwrap();
    let declaration: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/cargo/observe.json")).unwrap())
            .unwrap();
    assert_eq!(declaration["schema"], "chrono-host-cargo-observation/v1");
    assert_eq!(declaration["prerequisite_action"], "inputs_fetch");
    let mut environment = BTreeMap::new();
    for key in [
        "PATH",
        "HOME",
        "CARGO_HOME",
        "RUSTUP_HOME",
        "TMPDIR",
        "SDKROOT",
    ] {
        if let Ok(value) = std::env::var(key) {
            environment.insert(key.into(), value);
        }
    }
    environment.insert("RUSTUP_TOOLCHAIN".into(), "1.95.0".into());
    let tool = observation::tool(
        &root,
        "cargo",
        &["--version".into()],
        &environment,
        120,
        4 * 1024 * 1024,
    )
    .unwrap();
    observation::process_success(&tool.version).unwrap();
    assert_eq!(
        tool.version.stdout.trim(),
        "cargo 1.95.0 (f2d3ce0bd 2026-03-21)"
    );
    let directory = root.join(format!(
        ".chrono-harness/state/inputs/main-metadata-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("tool.json"),
        serde_json::to_vec_pretty(&tool).unwrap(),
    )
    .unwrap();
    let mut summaries = Vec::new();
    for identity in declaration["projects"].as_array().unwrap() {
        let identity = identity.as_str().unwrap();
        let row = projects["projects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == identity)
            .unwrap();
        let action = &row["actions"]["inputs_metadata"];
        assert_eq!(action["operation"], format!("inputs.metadata.{identity}"));
        assert_eq!(action["tool"], "cargo");
        let fetch = &row["actions"]["inputs_fetch"];
        assert_eq!(fetch["operation"], format!("inputs.fetch.{identity}"));
        assert_eq!(fetch["tool"], "cargo");
        let acquired = main_host_input_process(
            &root,
            &tool,
            fetch,
            &environment,
            &directory,
            &format!("{identity}.fetch"),
        );
        observation::process_success(&acquired).unwrap_or_else(|error| {
            panic!(
                "{identity} acquisition: {error}; original {}",
                directory.display()
            )
        });
        let process =
            main_host_input_process(&root, &tool, action, &environment, &directory, identity);
        observation::process_success(&process).unwrap_or_else(|error| {
            panic!(
                "{identity} metadata: {error}; original {}",
                directory.display()
            )
        });
        let metadata: serde_json::Value = serde_json::from_slice(&process.stdout_bytes).unwrap();
        assert_eq!(metadata["version"], 1);
        assert_eq!(
            metadata["workspace_root"],
            root.join(row["root"].as_str().unwrap()).to_str().unwrap()
        );
        summaries.push(json!({"project":identity,"stdout_sha256":process.stdout_sha256,"stderr_sha256":process.stderr_sha256,"packages":metadata["packages"].as_array().unwrap().len(),"resolution_nodes":metadata["resolve"]["nodes"].as_array().unwrap().len()}));
    }
    fs::write(
        directory.join("observations.json"),
        serde_json::to_vec_pretty(&summaries).unwrap(),
    )
    .unwrap();
    println!("main host original metadata: {}", directory.display());
    {
        let consumer = &declaration["native_consumer"];
        let context = main_host_native_context(consumer, &std::env::vars().collect()).unwrap();
        fs::write(
            directory.join("native-context.json"),
            serde_json::to_vec(&json!({"expectation": context})).unwrap(),
        )
        .unwrap();
        let guard = root.join(".chrono-harness/bin/chrono-judge-cargo");
        let filemap: serde_json::Value =
            serde_json::from_slice(&fs::read(root.join(".chrono-harness/FILEMAP.json")).unwrap())
                .unwrap();
        let bounds = &filemap["execution_plans"]["test:judge-cargo-tests"];
        // This child-local cutoff does not know time spent in preceding Cargo
        // binaries. Retain observed bytes during execution, even if the original
        // enclosing operation ends before this consumer can publish a terminal.
        let remaining = bounds["timeout_seconds"]
            .as_u64()
            .unwrap()
            .saturating_sub(started.elapsed().as_secs())
            .saturating_sub(30)
            .max(1);
        let out = run_process_observed_retained(
            &root,
            &main_host_native_spec(&root, consumer, remaining),
            &[],
            &chrono_harness::file_identity(&guard).unwrap().0,
            &ProcessEvidence {
                stdout: &directory.join("native-consumer.stdout"),
                stderr: &directory.join("native-consumer.stderr"),
                launch: &directory.join("native-consumer.launch.json"),
            },
        )
        .unwrap();
        assert_eq!(
            fs::read(directory.join("native-consumer.stdout")).unwrap(),
            out.stdout_bytes
        );
        assert_eq!(
            fs::read(directory.join("native-consumer.stderr")).unwrap(),
            out.stderr_bytes
        );
        retain_native_json(
            &directory.join("native-consumer.json"),
            &serde_json::to_vec(&out).unwrap(),
        );
        retain_native_json(&directory.join("native-terminal.json"), &serde_json::to_vec(&json!({"elapsed_millis":started.elapsed().as_millis(),"timeout_seconds":remaining,"exit_code":out.exit_code,"failure":out.failure,"stdout_sha256":out.stdout_sha256,"stderr_sha256":out.stderr_sha256})).unwrap());
        let phases: Vec<serde_json::Value> = out
            .stderr
            .lines()
            .filter_map(|line| line.strip_prefix("CHRONO_CARGO_PHASE "))
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        fs::write(
            directory.join("native-phases.json"),
            serde_json::to_vec(&phases).unwrap(),
        )
        .unwrap();
        if context["status"] == "failed" {
            assert_main_host_binding_rejection(&out, context);
            return;
        }
        assert_eq!(context["status"], "passed");
        assert_eq!(
            consumer["platform"],
            format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
        );
        observation::process_success(&out)
            .unwrap_or_else(|e| panic!("native guard: {e}; original {}", directory.display()));
        assert!(
            phases
                .iter()
                .any(|p| p["phase"] == "guard" && p["event"] == "terminal")
        );
        for phase in [
            "prepare-inputs",
            "native-host",
            "metadata",
            "validate-metadata",
            "consumer",
        ] {
            assert!(
                phases
                    .iter()
                    .any(|p| p["phase"] == phase && p["event"] == "terminal"),
                "missing original phase {phase}"
            );
        }
        assert_eq!(
            phases
                .iter()
                .filter(|p| p["phase"] == "unchanged-inputs" && p["event"] == "terminal")
                .count(),
            6
        );
        let report: serde_json::Value = serde_json::from_slice(&out.stdout_bytes).unwrap();
        assert!(
            out.exit_code == 0 && out.failure.is_none(),
            "{report} {}",
            String::from_utf8_lossy(&out.stderr_bytes)
        );
        assert_eq!(report["metadata"]["exit_code"], 0);
        assert_eq!(report["operation"]["exit_code"], 0);
        assert!(
            report["native_target"]["stdout"]
                .as_str()
                .unwrap()
                .contains("host: aarch64-apple-darwin")
        );
    }
}

fn main_host_native_context<'a>(
    consumer: &'a serde_json::Value,
    environment: &std::collections::BTreeMap<String, String>,
) -> Result<&'a serde_json::Value, String> {
    let source = environment
        .get("CHRONO_CHECK_SOURCE")
        .map(String::as_str)
        .unwrap_or("local");
    if !["local", "ci"].contains(&source) {
        return Err("unregistered check source".into());
    }
    let release_keys = [
        "CHRONO_RELEASE_RUN",
        "CHRONO_RELEASE_ATTEMPT",
        "CHRONO_RELEASE_JOB",
    ];
    let release = release_keys
        .iter()
        .any(|key| environment.contains_key(*key));
    let local_release = match environment.get("CHRONO_RELEASE_LOCAL").map(String::as_str) {
        None => false,
        Some("1") if release => true,
        _ => return Err("invalid local release context".into()),
    };
    if release
        && (release_keys
            .iter()
            .any(|key| environment.get(*key).is_none_or(String::is_empty))
            || environment["CHRONO_RELEASE_ATTEMPT"]
                .parse::<u64>()
                .unwrap_or(0)
                == 0)
    {
        return Err("incomplete release producer context".into());
    }
    if source == "ci" && local_release {
        return Err("conflicting check and release contexts".into());
    }
    let name = if source == "ci" || (release && !local_release) {
        "native-ci"
    } else {
        "local"
    };
    consumer["contexts"]
        .get(name)
        .ok_or_else(|| format!("missing declared {name} context"))
}

fn assert_main_host_binding_rejection(
    out: &chrono_harness::ProcessResult,
    expected: &serde_json::Value,
) {
    assert!(out.failure.is_none(), "{out:?}");
    assert_eq!(json!(out.exit_code), expected["exit_code"]);
    let report: serde_json::Value = serde_json::from_slice(&out.stdout_bytes).unwrap();
    assert_eq!(report["status"], expected["status"]);
    assert_eq!(report["exit_code"], expected["exit_code"]);
    assert_eq!(report["error"], expected["error"]);
    assert_eq!(report["input_closure_complete"], false);
    for field in [
        "tool",
        "compiler",
        "native_target",
        "linker",
        "toolchain",
        "metadata",
        "operation",
        "configuration",
    ] {
        assert!(report[field].is_null(), "{field}: {report}");
    }
    let phases: Vec<serde_json::Value> = out
        .stderr
        .lines()
        .filter_map(|line| line.strip_prefix("CHRONO_CARGO_PHASE "))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for phase in ["prepare-inputs", "guard"] {
        assert!(
            phases.iter().any(|p| p["phase"] == phase
                && p["event"] == "terminal"
                && p["outcome"] == "error"),
            "{phases:?}"
        );
    }
    assert!(
        phases
            .iter()
            .all(|p| p["phase"] == "guard" || p["phase"] == "prepare-inputs"),
        "{phases:?}"
    );
}

#[test]
fn main_host_declared_contexts_preserve_local_success_and_exact_binding_rejection() {
    use std::collections::BTreeMap;
    let root = fs::canonicalize(source()).unwrap();
    let declaration: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/cargo/observe.json")).unwrap())
            .unwrap();
    let consumer = &declaration["native_consumer"];
    let environment = |pairs: &[(&str, &str)]| -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).into(), (*v).into()))
            .collect()
    };
    for pairs in [
        vec![],
        vec![("CHRONO_CHECK_SOURCE", "local")],
        vec![
            ("CHRONO_RELEASE_RUN", "local-source"),
            ("CHRONO_RELEASE_ATTEMPT", "1"),
            ("CHRONO_RELEASE_JOB", "verify_judge_cargo_tests"),
            ("CHRONO_RELEASE_LOCAL", "1"),
        ],
    ] {
        assert_eq!(
            main_host_native_context(consumer, &environment(&pairs)).unwrap()["status"],
            "passed"
        );
    }
    for pairs in [
        vec![("CHRONO_CHECK_SOURCE", "ci")],
        vec![
            ("CHRONO_RELEASE_RUN", "native-source"),
            ("CHRONO_RELEASE_ATTEMPT", "1"),
            ("CHRONO_RELEASE_JOB", "linux_verify_judge_cargo_tests"),
        ],
        vec![
            ("CHRONO_RELEASE_RUN", "native-source"),
            ("CHRONO_RELEASE_ATTEMPT", "1"),
            ("CHRONO_RELEASE_JOB", "macos_verify_judge_cargo_tests"),
        ],
    ] {
        assert_eq!(
            main_host_native_context(consumer, &environment(&pairs)).unwrap()["status"],
            "failed"
        );
    }
    for pairs in [
        vec![("CHRONO_CHECK_SOURCE", "unknown")],
        vec![("CHRONO_RELEASE_JOB", "linux_verify_judge_cargo_tests")],
        vec![("CHRONO_RELEASE_LOCAL", "1")],
        vec![
            ("CHRONO_RELEASE_RUN", "native-source"),
            ("CHRONO_RELEASE_ATTEMPT", "0"),
            ("CHRONO_RELEASE_JOB", "macos_verify_judge_cargo_tests"),
        ],
        vec![
            ("CHRONO_CHECK_SOURCE", "ci"),
            ("CHRONO_RELEASE_RUN", "local-source"),
            ("CHRONO_RELEASE_ATTEMPT", "1"),
            ("CHRONO_RELEASE_JOB", "verify_judge_cargo_tests"),
            ("CHRONO_RELEASE_LOCAL", "1"),
        ],
    ] {
        assert!(main_host_native_context(consumer, &environment(&pairs)).is_err());
    }
    let expected =
        main_host_native_context(consumer, &environment(&[("CHRONO_CHECK_SOURCE", "ci")])).unwrap();
    let retained = root.join(".chrono-harness/state/inputs");
    fs::create_dir_all(&retained).unwrap();
    let directory = tempfile::Builder::new()
        .prefix("main-binding-rejection-")
        .tempdir_in(&retained)
        .unwrap()
        .keep();
    let mut config: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(CONFIG)).unwrap()).unwrap();
    let input = config["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|input| input["id"] == "tool.cargo")
        .unwrap();
    // The registered expected bytes remain fixed; this controlled host lacks them.
    let original_digest = input["sha256"].clone();
    input["location"] = json!(directory.join("absent-cargo"));
    assert_eq!(input["sha256"], original_digest);
    let config_path = directory.join("config.json");
    let relative_config = config_path.strip_prefix(&root).unwrap().to_str().unwrap();
    let mut projects: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join(config["registries"]["projects"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    let runner = projects["projects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|project| project["id"] == "runner")
        .unwrap();
    runner["actions"]["guarded_check"]["argv"][4] = json!(relative_config);
    let projects_path = directory.join("projects.json");
    fs::write(&projects_path, serde_json::to_vec(&projects).unwrap()).unwrap();
    config["registries"]["projects"] = json!(projects_path.strip_prefix(&root).unwrap());
    fs::write(&config_path, serde_json::to_vec(&config).unwrap()).unwrap();
    let mut spec = main_host_native_spec(&root, consumer, 120);
    let index = spec.args.iter().position(|arg| arg == "--config").unwrap();
    spec.args[index + 1] = relative_config.into();
    let out = chrono_harness::run_process_observed_retained(
        &root,
        &spec,
        &[],
        &chrono_harness::file_identity(std::path::Path::new(&spec.program))
            .unwrap()
            .0,
        &chrono_harness::ProcessEvidence {
            stdout: &directory.join("rejection.stdout"),
            stderr: &directory.join("rejection.stderr"),
            launch: &directory.join("rejection.launch.json"),
        },
    )
    .unwrap();
    fs::write(
        directory.join("rejection.json"),
        serde_json::to_vec(&out).unwrap(),
    )
    .unwrap();
    assert_main_host_binding_rejection(&out, expected);
}

fn main_host_input_process(
    root: &std::path::Path,
    tool: &chrono_harness::observation::Tool,
    action: &serde_json::Value,
    environment: &std::collections::BTreeMap<String, String>,
    directory: &std::path::Path,
    name: &str,
) -> chrono_harness::ProcessResult {
    let process = chrono_harness::run_process_observed_retained(
        root,
        &chrono_harness::CommandSpec {
            program: tool.path.to_str().unwrap().into(),
            args: serde_json::from_value(action["argv"].clone()).unwrap(),
            env: environment.clone(),
            timeout_seconds: 120,
            output_limit_bytes: 4 * 1024 * 1024,
        },
        &[],
        &tool.sha256,
        &chrono_harness::ProcessEvidence {
            stdout: &directory.join(format!("{name}.stdout")),
            stderr: &directory.join(format!("{name}.stderr")),
            launch: &directory.join(format!("{name}.launch.json")),
        },
    )
    .unwrap_or_else(|error| panic!("{name} launch: {error}; original {}", directory.display()));
    fs::write(
        directory.join(format!("{name}.json")),
        serde_json::to_vec(&process).unwrap(),
    )
    .unwrap();
    process
}

#[test]
fn main_host_offline_metadata_requires_registered_package_acquisition() {
    use chrono_harness::observation;
    use std::collections::BTreeMap;
    let root = fs::canonicalize(source()).unwrap();
    let projects: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/projects.json")).unwrap())
            .unwrap();
    let row = projects["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "runner")
        .unwrap();
    let retained = root.join(".chrono-harness/state/inputs");
    fs::create_dir_all(&retained).unwrap();
    let directory = tempfile::Builder::new()
        .prefix("main-metadata-acquisition-")
        .tempdir_in(&retained)
        .unwrap()
        .keep();
    // Package caches are rebuildable test artifacts, separate from the original
    // process evidence uploaded on failure. Their size must not consume its cap.
    let cargo_home = tempfile::Builder::new()
        .prefix("metadata-package-inventory-")
        .tempdir_in(root.join("crates/judge-cargo-tests/target"))
        .unwrap();
    let mut environment: BTreeMap<String, String> = std::env::vars()
        .filter(|(key, _)| {
            ["PATH", "HOME", "RUSTUP_HOME", "TMPDIR", "SDKROOT"].contains(&key.as_str())
        })
        .collect();
    environment.insert("RUSTUP_TOOLCHAIN".into(), "1.95.0".into());
    environment.insert(
        "CARGO_HOME".into(),
        cargo_home.path().to_str().unwrap().into(),
    );
    let tool = observation::tool(
        &root,
        "cargo",
        &["--version".into()],
        &environment,
        120,
        4 * 1024 * 1024,
    )
    .unwrap();
    observation::process_success(&tool.version).unwrap();
    let absent = main_host_input_process(
        &root,
        &tool,
        &row["actions"]["inputs_metadata"],
        &environment,
        &directory,
        "before",
    );
    assert!(
        observation::process_success(&absent).is_err(),
        "empty package inventory unexpectedly resolved"
    );
    assert!(
        absent.stderr.contains("no matching package") && absent.stderr.contains("offline"),
        "{}; original {}",
        absent.stderr,
        directory.display()
    );
    let fetch = &row["actions"]["inputs_fetch"];
    assert_eq!(fetch["operation"], "inputs.fetch.runner");
    let acquired = main_host_input_process(&root, &tool, fetch, &environment, &directory, "fetch");
    observation::process_success(&acquired).unwrap_or_else(|error| {
        panic!(
            "registered acquisition: {error}; original {}",
            directory.display()
        )
    });
    let resolved = main_host_input_process(
        &root,
        &tool,
        &row["actions"]["inputs_metadata"],
        &environment,
        &directory,
        "after",
    );
    observation::process_success(&resolved).unwrap_or_else(|error| {
        panic!(
            "offline resolution after acquisition: {error}; original {}",
            directory.display()
        )
    });
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&resolved.stdout_bytes).unwrap()["version"],
        1
    );
    println!(
        "controlled main-host acquisition originals: {}",
        directory.display()
    );
}

// Shared by the original native consumer and the enclosing-termination regression.
// Source registration remains the authority for its operation, policy and bound.
fn retain_native_json(path: &std::path::Path, bytes: &[u8]) {
    let mut original = tempfile::NamedTempFile::new_in(path.parent().unwrap()).unwrap();
    std::io::Write::write_all(&mut original, bytes).unwrap();
    original.persist_noclobber(path).unwrap();
}

fn main_host_native_spec(
    root: &std::path::Path,
    consumer: &serde_json::Value,
    timeout_seconds: u64,
) -> chrono_harness::CommandSpec {
    let policy: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(consumer["policy"].as_str().unwrap())).unwrap())
            .unwrap();
    chrono_harness::CommandSpec {
        program: root
            .join(".chrono-harness/bin/chrono-judge-cargo")
            .to_str()
            .unwrap()
            .into(),
        args: vec![
            "run".into(),
            "--host-root".into(),
            root.to_str().unwrap().into(),
            "--config".into(),
            ".chrono-harness/config.json".into(),
            "--policy".into(),
            consumer["policy"].as_str().unwrap().into(),
            "--operation".into(),
            consumer["operation"].as_str().unwrap().into(),
        ],
        // Preserve the original inherited environment and separate ownership carrier.
        env: std::env::vars().collect(),
        timeout_seconds,
        output_limit_bytes: policy["output_limit_bytes"].as_u64().unwrap() as usize,
    }
}

#[test]
fn main_host_native_enclosing_helper() {
    let Ok(directory) = std::env::var("CHRONO_NATIVE_ENCLOSING_FIXTURE") else {
        return;
    };
    let directory = std::path::Path::new(&directory);
    let root = fs::canonicalize(source()).unwrap();
    let declaration: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/cargo/observe.json")).unwrap())
            .unwrap();
    let filemap: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/FILEMAP.json")).unwrap())
            .unwrap();
    let spec = main_host_native_spec(
        &root,
        &declaration["native_consumer"],
        filemap["execution_plans"]["test:judge-cargo-tests"]["timeout_seconds"]
            .as_u64()
            .unwrap(),
    );
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let start = std::time::Instant::now();
            loop {
                let bytes = fs::read(directory.join("native-consumer.stderr")).unwrap_or_default();
                if bytes.starts_with(b"CHRONO_CARGO_PHASE ") && bytes.contains(&b'\n') {
                    // The exact consumer is interrupted only after its engine has
                    // retained a genuine native phase. No child exit is substituted.
                    #[cfg(target_os = "macos")]
                    let signal = 17; // SIGSTOP in the native macOS ABI.
                    #[cfg(not(target_os = "macos"))]
                    let signal = 19; // SIGSTOP in the supported Linux ABI.
                    assert_eq!(
                        unsafe { native_signal(std::process::id() as i32, signal) },
                        0
                    );
                    return;
                }
                if start.elapsed() >= std::time::Duration::from_secs(30) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });
        let out = chrono_harness::run_process_observed_retained(
            &root,
            &spec,
            &[],
            &chrono_harness::file_identity(std::path::Path::new(&spec.program))
                .unwrap()
                .0,
            &chrono_harness::ProcessEvidence {
                stdout: &directory.join("native-consumer.stdout"),
                stderr: &directory.join("native-consumer.stderr"),
                launch: &directory.join("native-consumer.launch.json"),
            },
        )
        .unwrap();
        retain_native_json(
            &directory.join("native-terminal.json"),
            &serde_json::to_vec(&out).unwrap(),
        );
    });
}

#[test]
fn main_host_native_partial_originals_survive_enclosing_termination() {
    let root = fs::canonicalize(source()).unwrap();
    let declaration: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join(".chrono-harness/cargo/observe.json")).unwrap())
            .unwrap();
    if declaration["native_consumer"]["platform"]
        != format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
    {
        return;
    }
    let directory = root.join(format!(
        ".chrono-harness/state/inputs/main-native-enclosing-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&directory).unwrap();
    let exe = std::env::current_exe().unwrap();
    let mut environment: std::collections::BTreeMap<String, String> = std::env::vars().collect();
    environment.insert(
        "CHRONO_NATIVE_ENCLOSING_FIXTURE".into(),
        directory.to_str().unwrap().into(),
    );
    let spec = chrono_harness::CommandSpec {
        program: exe.to_str().unwrap().into(),
        args: vec![
            "--exact".into(),
            "main_host_native_enclosing_helper".into(),
            "--nocapture".into(),
        ],
        env: environment,
        timeout_seconds: 10,
        output_limit_bytes: 4 * 1024 * 1024,
    };
    let outer = chrono_harness::run_process_observed(
        &root,
        &spec,
        &[],
        &chrono_harness::file_identity(&exe).unwrap().0,
    )
    .unwrap();
    fs::write(
        directory.join("enclosing-operation.json"),
        serde_json::to_vec(&outer).unwrap(),
    )
    .unwrap();
    println!(
        "main host enclosing native original: {}",
        directory.display()
    );
    assert!(
        outer
            .failure
            .as_deref()
            .unwrap()
            .starts_with("process timed out"),
        "{outer:?}"
    );
    assert_ne!(outer.exit_code, 0);
    let launch: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.join("native-consumer.launch.json")).unwrap())
            .unwrap();
    assert_eq!(launch["timeout_seconds"], 900);
    assert_eq!(
        launch["argv"][launch["argv"].as_array().unwrap().len() - 1],
        declaration["native_consumer"]["operation"]
    );
    assert_eq!(
        launch["sha256"],
        chrono_harness::file_identity(std::path::Path::new(launch["executable"].as_str().unwrap()))
            .unwrap()
            .0
    );
    let stderr = fs::read(directory.join("native-consumer.stderr")).unwrap();
    assert!(stderr.starts_with(b"CHRONO_CARGO_PHASE "));
    let first = String::from_utf8_lossy(&stderr);
    let phase: serde_json::Value = serde_json::from_str(
        first
            .lines()
            .next()
            .unwrap()
            .strip_prefix("CHRONO_CARGO_PHASE ")
            .unwrap(),
    )
    .unwrap();
    assert_eq!(phase["phase"], "guard");
    assert_eq!(phase["event"], "start");
    assert!(directory.join("native-consumer.stdout").is_file());
    assert!(
        !directory.join("native-terminal.json").exists(),
        "interrupted observer has no observed child terminal"
    );
    assert!(!directory.join("native-consumer.json").exists());
    for field in ["child_pid", "launcher_pid"] {
        assert_eq!(
            unsafe { native_signal(launch[field].as_i64().unwrap() as i32, 0) },
            -1,
            "{field} survived enclosing completion"
        );
        assert_eq!(std::io::Error::last_os_error().raw_os_error(), Some(3)); // ESRCH.
    }
    fs::write(directory.join("retained-observation.json"), serde_json::to_vec(&json!({"stdout_sha256":chrono_harness::file_identity(&directory.join("native-consumer.stdout")).unwrap().0,"stderr_sha256":chrono_harness::sha256(&stderr),"terminal_presence":"absent","child_terminal":"unobserved"})).unwrap()).unwrap();
}

unsafe extern "C" {
    #[link_name = "kill"]
    fn native_signal(pid: i32, signal: i32) -> i32;
}

#[test]
fn metadata_declaration_mismatch_retains_actual_successful_metadata_and_blocks_test() {
    let mut f = Fixture::new();
    f.contract["packages"][2]["features"] = json!([]);
    f.save();
    let (code, report, stderr) = f.call();
    assert_ne!(code, 0, "{report} {stderr}");
    assert_eq!(report["metadata"]["exit_code"], 0, "{report} {stderr}");
    assert!(report["error"].as_str().unwrap().contains("features"));
    assert!(report["operation"].is_null());
    assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
}

#[test]
fn ordered_configuration_arguments_are_not_collapsed_into_a_set() {
    let mut f = Fixture::new();
    for id in ["first", "second"] {
        let path = format!(".chrono-harness/cargo/{id}.toml");
        write(&f.root(), &path, "[build]\nrustflags=[]\n");
        let digest = sha256(&fs::read(f.root().join(&path)).unwrap());
        f.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":id,"location":path,"sha256":digest}));
        f.values.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap()
            .push(edge(&format!("input:{id}"), "runtime-input", "project:t"));
        f.contract["configuration_files"]
            .as_array_mut()
            .unwrap()
            .push(json!({"path":path,"input":id}));
        f.contract["metadata"]["argv"]
            .as_array_mut()
            .unwrap()
            .extend([json!("--config"), json!(path)]);
    }
    for id in ["second", "first"] {
        f.contract["operations"]["test.t"]["argv"]
            .as_array_mut()
            .unwrap()
            .extend([
                json!("--config"),
                json!(format!(".chrono-harness/cargo/{id}.toml")),
            ]);
    }
    f.save();
    let (code, report, stderr) = f.call();
    assert_ne!(code, 0, "{report} {stderr}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("ordered configuration")
    );
    assert!(report["metadata"].is_null() && report["operation"].is_null());
    let argv = f.contract["operations"]["test.t"]["argv"]
        .as_array_mut()
        .unwrap();
    let length = argv.len();
    argv.swap(length - 3, length - 1);
    f.save();
    let (code, report, stderr) = f.call();
    assert_eq!(code, 0, "ordered positive control: {report} {stderr}");
    assert_eq!(report["operation"]["exit_code"], 0);
}

#[test]
fn changed_missing_disconnected_and_unlisted_package_inputs_block_before_metadata() {
    for case in ["changed", "missing", "edge", "inventory"] {
        let mut f = Fixture::new();
        match case {
            "changed" => write(
                &f.root(),
                ".chrono-harness/state/vendor/fixture-dep/src/lib.rs",
                "changed bytes\n",
            ),
            "missing" => f.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
                .as_array_mut()
                .unwrap()
                .retain(|i| i["id"] != "dep.lib"),
            "edge" => f.values.get_mut(FM).unwrap()["project_edges"]
                .as_array_mut()
                .unwrap()
                .retain(|e| e["from"] != "input:dep.lib"),
            _ => write(
                &f.root(),
                ".chrono-harness/state/vendor/fixture-dep/src/unlisted.rs",
                "pub fn hidden() {}\n",
            ),
        }
        f.save();
        let (code, report, stderr) = f.call();
        assert_ne!(code, 0, "{case}: {report} {stderr}");
        assert!(
            report["error"].as_str().unwrap().contains("E_CARGO_INPUT"),
            "{case}: {report}"
        );
        assert!(
            report["metadata"].is_null() && report["operation"].is_null(),
            "{case}: {report}"
        );
        assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
    }
}

#[test]
fn actual_resolution_packages_aliases_and_dependency_kinds_are_checked() {
    for case in ["packages", "alias", "kind"] {
        let mut f = Fixture::new();
        match case {
            "packages" => {
                f.contract["packages"].as_array_mut().unwrap().pop();
                f.contract["packages"][2]["dependencies"] = json!([]);
            }
            "alias" => {
                f.contract["packages"][0]["dependencies"][0]["name"] = "another_alias".into()
            }
            _ => f.contract["packages"][0]["dependencies"][0]["kinds"][0]["kind"] = "dev".into(),
        }
        f.save();
        let (code, report, stderr) = f.call();
        assert_ne!(code, 0, "{case}: {report} {stderr}");
        assert_eq!(
            report["metadata"]["exit_code"], 0,
            "{case}: {report} {stderr}"
        );
        assert!(report["operation"].is_null());
        assert!(
            report["error"]
                .as_str()
                .unwrap()
                .contains(if case == "packages" {
                    "package set"
                } else {
                    "resolved dependencies"
                })
        );
        assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
    }
}

#[test]
fn lock_checksum_and_actual_source_location_cannot_be_substituted() {
    let mut f = Fixture::new();
    f.contract["packages"][2]["checksum"] = "c".repeat(64).into();
    f.save();
    let (code, report, stderr) = f.call();
    assert_ne!(code, 0, "{report} {stderr}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("lock identity/checksum")
    );
    assert!(report["metadata"].is_null());
    let mut f = Fixture::new();
    for id in ["dep.manifest", "dep.lib", "dep.checksums"] {
        let input = f.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|x| x["id"] == id)
            .unwrap();
        let old = input["location"].as_str().unwrap().to_string();
        let new = old.replace("/vendor/", "/retained-copy/");
        let root = fs::canonicalize(f.dir.path()).unwrap();
        let target = root.join(&new);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(root.join(old), target).unwrap();
        input["location"] = new.into();
    }
    f.save();
    let (code, report, stderr) = f.call();
    assert_ne!(code, 0, "{report} {stderr}");
    assert_eq!(report["metadata"]["exit_code"], 0, "{report} {stderr}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("source location mismatch")
    );
    assert!(report["operation"].is_null());
}

#[test]
fn registered_entry_and_observed_tool_version_are_required() {
    for case in ["entry", "version", "offline"] {
        let mut f = Fixture::new();
        match case {
            "entry" => {
                f.values.get_mut(PROJECTS).unwrap()["projects"][1]["actions"]["execute"]["argv"]
                    [0] = "something-else".into()
            }
            "version" => {
                f.values.get_mut(CONFIG).unwrap()["tools"][0]["expected_version"] =
                    "not the actual cargo version".into()
            }
            _ => f.contract["metadata"]["argv"]
                .as_array_mut()
                .unwrap()
                .retain(|a| a != "--offline"),
        }
        f.save();
        let (code, report, stderr) = f.call();
        assert_ne!(code, 0, "{case}: {report} {stderr}");
        assert!(report["metadata"].is_null() && report["operation"].is_null());
        if case == "version" {
            assert_eq!(report["tool"]["version"]["exit_code"], 0);
            assert!(
                report["tool"]["version"]["stdout"]
                    .as_str()
                    .unwrap()
                    .starts_with("cargo 1.95.0")
            );
        }
        assert!(
            report["error"].as_str().unwrap().contains(match case {
                "entry" => "registered Cargo guard entry",
                "version" => "E_TOOL_BINDING",
                _ => "locked offline",
            }),
            "{case}: {report}"
        );
    }
}

#[test]
fn failing_tests_and_post_execution_input_changes_retain_real_receipts() {
    for mutate in [false, true] {
        let mut f = Fixture::new();
        if mutate {
            f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CHRONO_MUTATION_TARGET"] =
                f.root()
                    .join(".chrono-harness/state/vendor/fixture-dep/src/lib.rs")
                    .to_str()
                    .unwrap()
                    .into();
            write(
                &f.root(),
                "t/src/lib.rs",
                "#[test] fn actual_external_dependency() { assert_eq!(p::value(),43); std::fs::write(std::env::var(\"CHRONO_MUTATION_TARGET\").unwrap(),b\"changed after compilation\").unwrap(); }\n",
            );
        } else {
            write(
                &f.root(),
                "t/src/lib.rs",
                "#[test] fn actual_external_dependency() { assert_eq!(p::value(),44); }\n",
            );
        }
        f.save();
        let (code, report, stderr) = f.call();
        assert_ne!(code, 0, "{report} {stderr}");
        assert_eq!(report["metadata"]["exit_code"], 0);
        assert_eq!(
            report["operation"]["exit_code"],
            if mutate { 0 } else { 101 }
        );
        if mutate {
            assert!(
                report["error"]
                    .as_str()
                    .unwrap()
                    .contains("input changed during guarded operation")
            );
        } else {
            assert_eq!(code, 101);
            assert!(
                report["operation"]["stdout"]
                    .as_str()
                    .unwrap()
                    .contains("1 failed")
            );
        }
        for field in ["metadata", "operation"] {
            for stream in ["stdout", "stderr"] {
                let bytes: Vec<u8> =
                    serde_json::from_value(report[field][format!("{stream}_bytes")].clone())
                        .unwrap();
                assert_eq!(report[field][format!("{stream}_sha256")], sha256(&bytes));
            }
        }
    }
}

#[test]
fn connected_absence_is_checked_before_and_after_cargo_and_cannot_supply_package_bytes() {
    for case in ["stable", "present", "package", "mutation", "symlink"] {
        let mut f = Fixture::new();
        let path = absent_input(&mut f);
        match case {
            "stable" => {}
            "present" => fs::write(&path, []).unwrap(),
            "package" => {
                f.contract["packages"][2]["manifest_input"] = json!("optional");
                f.contract["packages"][2]["inputs"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!("optional"));
            }
            "mutation" => {
                f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CHRONO_MUTATE"] =
                    json!(path);
                write(
                    &f.root(),
                    "t/src/lib.rs",
                    "#[test] fn create_absent_input() { std::fs::write(std::env::var(\"CHRONO_MUTATE\").unwrap(),b\"created\").unwrap(); }\n",
                );
            }
            "symlink" => {
                #[cfg(unix)]
                std::os::unix::fs::symlink(path.with_extension("missing"), &path).unwrap();
                #[cfg(not(unix))]
                continue;
            }
            _ => unreachable!(),
        }
        f.save();
        let (exit, r, err) = f.call();
        if case == "stable" {
            assert_eq!(exit, 0, "{r} {err}");
            assert_eq!(r["operation"]["exit_code"], 0);
        } else {
            assert_ne!(exit, 0, "{case}: {r} {err}");
            let expected = match case {
                "present" => "must be absent",
                "package" => "requires bound present bytes",
                "mutation" => "absent input changed during guarded operation",
                _ => "symlink input",
            };
            assert!(format!("{r} {err}").contains(expected), "{case}: {r} {err}");
            if case == "mutation" {
                assert_eq!(r["operation"]["exit_code"], 0);
                assert_eq!(fs::read(path).unwrap(), b"created");
            } else {
                assert!(r["metadata"].is_null());
            }
        }
    }
}
#[test]
fn compiler_binding_runs_real_cargo_with_declared_executable_inputs() {
    let mut f = Fixture::new();
    compiler_binding(&mut f);
    let (exit, report, stderr) = f.call();
    assert_eq!(exit, 0, "{report} {stderr}");
    assert_eq!(report["metadata"]["exit_code"], 0);
    assert_eq!(report["operation"]["exit_code"], 0);
    assert_eq!(
        report["compiler"]["path"],
        f.values[CONFIG]["environment"]["values"]["RUSTC"]
    );
    let input = f.values[CONFIG]["environment"]["inputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|i| i["id"] == "tool.rustc")
        .unwrap();
    assert_eq!(report["compiler"]["sha256"], input["sha256"]);
    assert_eq!(report["input_closure_complete"], false);
    assert_eq!(
        fs::read(f.root().join(".chrono-harness/state/test-ran")).unwrap(),
        b"ran"
    );
}

#[test]
fn compiler_binding_rejects_unbound_tools_and_selection_overrides_before_metadata() {
    let mut f = Fixture::new();
    compiler_binding(&mut f);
    let values = f.values.clone();
    let contract = f.contract.clone();
    for case in [
        "missing-input",
        "disconnected",
        "digest",
        "tool",
        "version",
        "selection",
        "wrapper",
        "workspace-wrapper",
        "missing-wrapper",
        "clippy",
    ] {
        f.values = values.clone();
        f.contract = contract.clone();
        match case {
            "missing-input" => f.contract["cargo_input"] = json!("missing"),
            "disconnected" => f.values.get_mut(FM).unwrap()["project_edges"]
                .as_array_mut()
                .unwrap()
                .retain(|e| e["from"] != "input:tool.rustc"),
            "digest" => {
                f.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|i| i["id"] == "tool.rustc")
                    .unwrap()["sha256"] = json!("0".repeat(64))
            }
            "tool" => {
                f.values.get_mut(CONFIG).unwrap()["tools"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|t| t["id"] == "rustc")
                    .unwrap()["program"] = json!("/bin/sh")
            }
            "version" => {
                f.values.get_mut(CONFIG).unwrap()["tools"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|t| t["id"] == "rustc")
                    .unwrap()["expected_version"] = json!("wrong compiler version")
            }
            "selection" => {
                f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["RUSTC"] =
                    json!("/bin/false")
            }
            "wrapper" => {
                f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["RUSTC_WRAPPER"] =
                    json!("/bin/false")
            }
            "workspace-wrapper" => {
                f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["RUSTC_WORKSPACE_WRAPPER"] =
                    json!("/bin/false")
            }
            "missing-wrapper" => {
                f.values.get_mut(CONFIG).unwrap()["environment"]["values"]
                    .as_object_mut()
                    .unwrap()
                    .remove("RUSTC_WRAPPER");
            }
            "clippy" => f.contract["operations"]["test.t"]["argv"][0] = json!("clippy"),
            _ => unreachable!(),
        }
        f.save();
        let (exit, report, stderr) = f.call();
        assert_ne!(exit, 0, "{case}: {report} {stderr}");
        assert!(
            report["error"].as_str().is_some_and(|e| e.contains("input")
                || e.contains("compiler")
                || e.contains("tool")
                || e.contains("version")
                || e.contains("wrapper")),
            "{case}: {report}"
        );
        assert!(
            report["metadata"].is_null() && report["operation"].is_null(),
            "{case}: {report}"
        );
        assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
    }
}
#[test]
fn compiler_binding_rejects_cargo_configuration_selection_overrides() {
    for case in ["build", "env", "include", "alias"] {
        let mut f = Fixture::new();
        compiler_binding(&mut f);
        let path = f.root().join(".chrono-harness/cargo/config.toml");
        let original = fs::read_to_string(&path).unwrap();
        match case {
            "build" => f.configuration(
                "cargo.config",
                &path,
                &(original + "\n[build]\nrustc='/bin/sh'\n"),
            ),
            "env" => f.configuration(
                "cargo.config",
                &path,
                &(original + "\n[env]\nRUSTC={value='/bin/sh',force=true}\n"),
            ),
            "include" => {
                let included = f.root().join(".chrono-harness/cargo/selection.toml");
                f.configuration(
                    "selection",
                    &included,
                    "[env]\nRUSTC_WRAPPER={value='/bin/sh',force=true}\n",
                );
                f.configuration(
                    "cargo.config",
                    &path,
                    &format!("include=['selection.toml']\n{original}"),
                );
            }
            "alias" => {
                f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CARGO_BUILD_RUSTC"] =
                    json!("/bin/sh")
            }
            _ => unreachable!(),
        }
        f.save();
        let (exit, report, stderr) = f.call();
        assert_ne!(exit, 0, "{case}: {report} {stderr}");
        assert!(
            report["error"]
                .as_str()
                .unwrap()
                .contains("compiler selection"),
            "{case}: {report}"
        );
        assert!(report["metadata"].is_null() && report["operation"].is_null());
    }
}
#[test]
fn compiler_binding_tracks_real_invocation_and_rejects_post_execution_mutation() {
    use std::os::unix::fs::PermissionsExt;
    let mut f = Fixture::new();
    compiler_binding(&mut f);
    let actual = f.values[CONFIG]["environment"]["values"]["RUSTC"]
        .as_str()
        .unwrap()
        .to_owned();
    let wrapper = f.root().join(".chrono-harness/state/bound-compiler");
    let trace = f.root().join(".chrono-harness/state/compiler-arguments");
    let quote = |s: &str| format!("'{}'", s.replace('\'', "'\"'\"'"));
    let script = format!(
        "#!/bin/sh\nprintf '%s\\n' \"$@\" >> {}\nexec {} \"$@\"\n",
        quote(trace.to_str().unwrap()),
        quote(&actual)
    );
    fs::write(&wrapper, &script).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    f.values.get_mut(CONFIG).unwrap()["tools"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|t| t["id"] == "rustc")
        .unwrap()["program"] = json!(wrapper);
    let input = f.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i["id"] == "tool.rustc")
        .unwrap();
    input["location"] = json!(wrapper);
    input["sha256"] = json!(sha256(script.as_bytes()));
    f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["RUSTC"] = json!(wrapper);
    f.contract["operations"]["test.t"]["argv"]
        .as_array_mut()
        .unwrap()
        .push(json!("--lib"));
    f.save();
    let (exit, r, stderr) = f.call();
    assert_eq!(exit, 0, "{r} {stderr}");
    assert!(
        fs::read_to_string(&trace)
            .unwrap()
            .contains("--crate-name\nt\n")
    );
    f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CHRONO_MUTATE"] = json!(wrapper);
    write(
        &f.root(),
        "t/src/lib.rs",
        "#[test] fn mutate_selected_compiler() { std::fs::write(std::env::var(\"CHRONO_MUTATE\").unwrap(), b\"changed\").unwrap(); }\n",
    );
    f.save();
    let (exit, r, stderr) = f.call();
    assert_ne!(exit, 0, "{r} {stderr}");
    assert_eq!(r["operation"]["exit_code"], 0);
    assert!(
        r["error"].as_str().unwrap().contains("input changed"),
        "{r}"
    );
    assert_eq!(fs::read(wrapper).unwrap(), b"changed");
}

#[test]
fn compiler_binding_requires_versioned_present_bindings_without_legacy_reinterpretation() {
    let mut f = Fixture::new();
    compiler_binding(&mut f);
    let contract = f.contract.clone();
    for version in [2, 3, 4] {
        for field in ["cargo_input", "compiler"] {
            for null in [false, true] {
                f.contract = contract.clone();
                f.contract["schema"] = json!(format!("chrono-cargo-inputs/v{version}"));
                if version == 2 {
                    f.contract
                        .as_object_mut()
                        .unwrap()
                        .remove("configuration_ancestors");
                }
                if version < 4 {
                    f.contract
                        .as_object_mut()
                        .unwrap()
                        .remove(if field == "compiler" {
                            "cargo_input"
                        } else {
                            "compiler"
                        });
                }
                if null {
                    f.contract[field] = json!(null);
                } else if version == 4 {
                    f.contract.as_object_mut().unwrap().remove(field);
                }
                f.save();
                let (exit, r, stderr) = f.call();
                assert_ne!(exit, 0, "v{version} {field} null={null}: {r} {stderr}");
                assert!(
                    r["error"].as_str().unwrap().contains("contract schema"),
                    "{r}"
                );
                assert!(r["metadata"].is_null() && r["operation"].is_null());
            }
        }
    }
    f.contract = contract;
    absent_input(&mut f);
    f.contract["compiler"]["input"] = json!("optional");
    f.save();
    let (exit, r, stderr) = f.call();
    assert_ne!(exit, 0, "{r} {stderr}");
    assert!(
        r["error"]
            .as_str()
            .unwrap()
            .contains("requires bound present bytes"),
        "{r}"
    );
    assert!(r["metadata"].is_null() && r["operation"].is_null());
    assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
}

#[test]
fn toolchain_binding_records_sysroot_backend_linker_sdk_and_build_inputs() {
    let mut f = Fixture::new();
    toolchain_binding(&mut f);
    let (exit, report, stderr) = f.call();
    assert_eq!(exit, 0, "{report} {stderr}");
    assert_eq!(report["schema"], "chrono-cargo-run/v3");
    assert_eq!(report["toolchain"]["sysroot"]["files"], 1);
    assert_eq!(report["toolchain"]["backend"], json!(["toolchain.backend"]));
    assert_eq!(report["toolchain"]["sdks"][0]["files"], 1);
    assert_eq!(
        report["toolchain"]["build_script_inputs"],
        json!(["toolchain.build-script"])
    );
    let linker_environment = format!(
        "CARGO_TARGET_{}_LINKER",
        f.contract["target"]
            .as_str()
            .unwrap()
            .replace('-', "_")
            .to_uppercase()
    );
    assert_eq!(
        report["toolchain"]["linker_environment"],
        linker_environment
    );
    assert!(report["linker"]["path"].is_string(), "{report}");
    assert_eq!(report["operation"]["exit_code"], 0);
}

#[test]
fn toolchain_binding_rejects_selection_and_inventory_drift_before_cargo() {
    let mut f = Fixture::new();
    toolchain_binding(&mut f);
    f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["RUSTFLAGS"] =
        json!("--sysroot=/tmp/not-the-registered-sysroot");
    f.save();
    let (exit, report, stderr) = f.call();
    assert_ne!(exit, 0, "{report} {stderr}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("exact RUSTFLAGS sysroot selection"),
        "{report}"
    );
    assert!(report["metadata"].is_null() && report["operation"].is_null());

    let mut f = Fixture::new();
    toolchain_binding(&mut f);
    let manifest = f.root().join(".chrono-harness/state/sdk.json");
    let mut value: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    value["files"][0]["sha256"] =
        json!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    fs::write(&manifest, serde_json::to_vec(&value).unwrap()).unwrap();
    let (exit, report, stderr) = f.call();
    assert_ne!(exit, 0, "{report} {stderr}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("input identity changed"),
        "{report}"
    );
    assert!(report["metadata"].is_null() && report["operation"].is_null());
}

#[cfg(unix)]
fn sdk_links(f: &mut Fixture) -> serde_json::Value {
    use std::os::unix::fs::symlink;
    toolchain_binding(f);
    let root = f.root();
    let sdk = root.join(".chrono-harness/state/sdk");
    fs::create_dir(sdk.join("include")).unwrap();
    fs::write(sdk.join("include/header"), "registered header").unwrap();
    symlink("SDKROOT.marker", sdk.join("alias")).unwrap();
    symlink("alias", sdk.join("chain")).unwrap();
    symlink("include", sdk.join("headers")).unwrap();
    json!({
        "schema":"chrono-input-directory/v2",
        "root":".chrono-harness/state/sdk",
        "files":[
            {"path":"SDKROOT.marker","sha256":sha256(b"declared sdk\n")},
            {"path":"include/header","sha256":sha256(b"registered header")}
        ],
        "symlinks":[
            {"path":"alias","target":"SDKROOT.marker","kind":"file"},
            {"path":"chain","target":"alias","kind":"file"},
            {"path":"headers","target":"include","kind":"directory"}
        ]
    })
}

#[cfg(unix)]
fn save_sdk_links(f: &mut Fixture, manifest: &serde_json::Value) {
    let bytes = serde_json::to_vec(manifest).unwrap();
    fs::write(f.root().join(".chrono-harness/state/sdk.json"), &bytes).unwrap();
    let input = f.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["id"] == "toolchain.sdk.manifest")
        .unwrap();
    input["sha256"] = json!(sha256(&bytes));
    f.save();
}

#[cfg(unix)]
#[test]
fn v3_directory_aliases_cover_empty_and_alias_only_directories_and_detect_drift() {
    use std::os::unix::fs::symlink;
    for case in ["accepted", "v2", "missing", "extra-child", "mutation"] {
        let mut f = Fixture::new();
        let mut manifest = sdk_links(&mut f);
        let sdk = f.root().join(".chrono-harness/state/sdk");
        fs::create_dir(sdk.join("empty")).unwrap();
        fs::create_dir(sdk.join("container")).unwrap();
        symlink("../empty", sdk.join("container/leaf")).unwrap();
        symlink("container", sdk.join("container-alias")).unwrap();
        manifest["schema"] = json!("chrono-input-directory/v3");
        manifest["directories"] =
            json!([{"path":"empty","entries":[]},{"path":"container","entries":["leaf"]}]);
        manifest["symlinks"].as_array_mut().unwrap().extend([
            json!({"path":"container/leaf","target":"../empty","kind":"directory"}),
            json!({"path":"container-alias","target":"container","kind":"directory"}),
        ]);
        match case {
            "v2" => manifest["schema"] = json!("chrono-input-directory/v2"),
            "missing" => {
                manifest.as_object_mut().unwrap().remove("directories");
            }
            "extra-child" => fs::write(sdk.join("empty/unlisted"), b"changed").unwrap(),
            "mutation" => {
                f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CHRONO_MUTATE"] =
                    json!(sdk.join("empty/unlisted"));
                write(
                    &f.root(),
                    "t/src/lib.rs",
                    "#[test] fn change_directory() { std::fs::write(std::env::var(\"CHRONO_MUTATE\").unwrap(),b\"changed\").unwrap(); }\n",
                );
            }
            _ => {}
        }
        save_sdk_links(&mut f, &manifest);
        let (exit, report, stderr) = f.call();
        if case == "accepted" {
            assert_eq!(exit, 0, "{report} {stderr}");
        } else {
            assert_ne!(exit, 0, "{case}: {report} {stderr}");
            if case == "mutation" {
                assert_eq!(report["operation"]["exit_code"], 0, "{report}");
                assert!(
                    report["error"]
                        .as_str()
                        .unwrap()
                        .contains("directory children differ"),
                    "{report}"
                );
            } else {
                assert!(report["operation"].is_null());
            }
        }
    }
}

#[cfg(unix)]
#[test]
fn directory_links_are_explicit_and_consumed_by_the_real_compiler() {
    let mut f = Fixture::new();
    let mut manifest = sdk_links(&mut f);
    let alias = f.root().join(".chrono-harness/state/sdk/alias");
    fs::remove_file(&alias).unwrap();
    std::os::unix::fs::symlink("./SDKROOT.marker", alias).unwrap();
    manifest["symlinks"][0]["target"] = json!("./SDKROOT.marker");
    save_sdk_links(&mut f, &manifest);
    let lib = f.root().join("p/src/lib.rs");
    let mut source = fs::read_to_string(&lib).unwrap();
    source.push_str("\nconst _: &str = include_str!(concat!(env!(\"CHRONO_SDK_ROOT\"), \"/chain\"));\nconst _: &str = include_str!(concat!(env!(\"CHRONO_SDK_ROOT\"), \"/headers/header\"));\n");
    fs::write(lib, source).unwrap();
    let (exit, report, stderr) = f.call();
    assert_eq!(exit, 0, "{report} {stderr}");
    assert_eq!(report["toolchain"]["sdks"][0]["files"], 2);
    assert_eq!(report["operation"]["exit_code"], 0);
    assert!(f.root().join(".chrono-harness/state/test-ran").exists());
}

#[cfg(unix)]
#[test]
fn directory_links_reject_missing_registration_escape_cycles_and_wrong_kinds() {
    use std::os::unix::fs::symlink;
    for case in [
        "undeclared",
        "escape",
        "cycle",
        "kind",
        "uncovered",
        "literal",
        "v1",
        "v1-null",
        "v2-null",
        "v2-missing",
    ] {
        let mut f = Fixture::new();
        let mut manifest = sdk_links(&mut f);
        let sdk = f.root().join(".chrono-harness/state/sdk");
        match case {
            "undeclared" => {
                manifest["symlinks"].as_array_mut().unwrap().remove(0);
            }
            "escape" | "cycle" | "uncovered" => {
                let target = match case {
                    "escape" => "../../../../outside",
                    "cycle" => "chain",
                    _ => "unregistered-marker",
                };
                fs::write(sdk.join("unregistered-marker"), "unregistered").unwrap();
                fs::remove_file(sdk.join("alias")).unwrap();
                symlink(target, sdk.join("alias")).unwrap();
                manifest["symlinks"][0]["target"] = json!(target);
            }
            "kind" => manifest["symlinks"][0]["kind"] = json!("directory"),
            "literal" => manifest["symlinks"][0]["target"] = json!("./SDKROOT.marker"),
            "v1" => manifest["schema"] = json!("chrono-input-directory/v1"),
            "v1-null" => {
                manifest["schema"] = json!("chrono-input-directory/v1");
                manifest["symlinks"] = serde_json::Value::Null;
            }
            "v2-null" => manifest["symlinks"] = serde_json::Value::Null,
            "v2-missing" => {
                manifest.as_object_mut().unwrap().remove("symlinks");
            }
            _ => unreachable!(),
        }
        save_sdk_links(&mut f, &manifest);
        let (exit, report, stderr) = f.call();
        assert_ne!(exit, 0, "{case}: {report} {stderr}");
        assert!(
            report["error"].as_str().unwrap().contains("directory"),
            "{case}: {report}"
        );
        assert!(
            report["metadata"].is_null() && report["operation"].is_null(),
            "{case}: {report}"
        );
        assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
    }
}

#[cfg(unix)]
#[test]
fn directory_link_drift_preserves_the_successful_consumer_receipt() {
    let mut f = Fixture::new();
    let manifest = sdk_links(&mut f);
    save_sdk_links(&mut f, &manifest);
    let lib = f.root().join("t/src/lib.rs");
    let mut source = fs::read_to_string(&lib).unwrap();
    source.push_str(
        r#"
#[test]
fn change_declared_link() {
    let sdk = std::path::PathBuf::from(std::env::var("CHRONO_SDK_ROOT").unwrap());
    std::fs::remove_file(sdk.join("alias")).unwrap();
    std::os::unix::fs::symlink("./SDKROOT.marker", sdk.join("alias")).unwrap();
}
"#,
    );
    fs::write(lib, source).unwrap();
    let (exit, report, stderr) = f.call();
    assert_ne!(exit, 0, "{report} {stderr}");
    assert_eq!(report["operation"]["exit_code"], 0, "{report} {stderr}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("directory link target changed"),
        "{report}"
    );
}

#[cfg(unix)]
#[test]
fn directory_root_retargeting_preserves_the_successful_consumer_receipt() {
    use std::os::unix::fs::symlink;
    let mut f = Fixture::new();
    let manifest = sdk_links(&mut f);
    save_sdk_links(&mut f, &manifest);
    let state = f.root().join(".chrono-harness/state");
    fs::rename(state.join("sdk"), state.join("sdk-original")).unwrap();
    symlink("sdk-original", state.join("sdk")).unwrap();
    f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CHRONO_SDK_ROOT"] =
        json!(state.join("sdk-original"));
    f.save();
    fs::create_dir_all(state.join("sdk-other/include")).unwrap();
    for file in ["SDKROOT.marker", "include/header"] {
        fs::copy(
            state.join("sdk-original").join(file),
            state.join("sdk-other").join(file),
        )
        .unwrap();
    }
    for (path, target) in [
        ("alias", "SDKROOT.marker"),
        ("chain", "alias"),
        ("headers", "include"),
    ] {
        symlink(target, state.join("sdk-other").join(path)).unwrap();
    }
    let lib = f.root().join("t/src/lib.rs");
    let mut source = fs::read_to_string(&lib).unwrap();
    source.push_str(
        r#"
#[test]
fn change_directory_root() {
    let sdk = std::path::PathBuf::from(std::env::var("CHRONO_SDK_ROOT").unwrap());
    let declared = sdk.parent().unwrap().join("sdk");
    std::fs::remove_file(&declared).unwrap();
    std::os::unix::fs::symlink("sdk-other", &declared).unwrap();
}
"#,
    );
    fs::write(lib, source).unwrap();
    let (exit, report, stderr) = f.call();
    assert_ne!(exit, 0, "{report} {stderr}");
    assert_eq!(report["operation"]["exit_code"], 0, "{report} {stderr}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("directory root changed"),
        "{report}"
    );
    assert!(state.join("sdk-original/SDKROOT.marker").is_file());
}

#[cfg(unix)]
#[test]
fn v1_directory_file_replacement_by_same_content_alias_preserves_consumer_result() {
    let mut f = Fixture::new();
    toolchain_binding(&mut f);
    let lib = f.root().join("t/src/lib.rs");
    let mut source = fs::read_to_string(&lib).unwrap();
    source.push_str(
        r#"
#[test]
fn replace_declared_regular_file() {
    let sdk = std::path::PathBuf::from(std::env::var("CHRONO_SDK_ROOT").unwrap());
    std::fs::rename(sdk.join("SDKROOT.marker"), sdk.join("same-content")).unwrap();
    std::os::unix::fs::symlink("same-content", sdk.join("SDKROOT.marker")).unwrap();
}
"#,
    );
    fs::write(lib, source).unwrap();
    let (exit, report, stderr) = f.call();
    assert_ne!(exit, 0, "{report} {stderr}");
    assert_eq!(report["operation"]["exit_code"], 0, "{report} {stderr}");
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("directory inventory contains symlink"),
        "{report}"
    );
}
