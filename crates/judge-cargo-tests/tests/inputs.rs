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
