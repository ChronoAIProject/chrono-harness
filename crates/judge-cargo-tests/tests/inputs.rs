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
    // Cargo canonicalizes a file URL in the lockfile. Encode the actual UTF-8
    // allocation path so the declared source has that same identity, including
    // the adopted parent's spaces and Unicode.
    let path = fs::canonicalize(upstream.path()).unwrap();
    let encoded: String = path
        .to_str()
        .unwrap()
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"/-_.~".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect();
    let url = format!("file://{encoded}");
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
