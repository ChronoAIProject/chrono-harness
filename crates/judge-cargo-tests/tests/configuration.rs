#[path = "support/cargo.rs"]
mod cargo;
use cargo::*;
use serde_json::json;
use std::fs;

fn portable(f: &mut Fixture) {
    let root = f.root();
    f.contract["schema"] = "chrono-cargo-inputs/v3".into();
    f.contract["configuration_ancestors"] = "absent".into();
    f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CARGO_HOME"] =
        ".chrono-harness/state/cargo-home".into();
    f.configuration(
        "cargo.config",
        &root.join(".chrono-harness/cargo/config.toml"),
        "[source.crates-io]\nreplace-with='fixtures'\n[source.fixtures]\ndirectory='state/vendor'\n",
    );
    let rows = f.contract["configuration_files"].as_array_mut().unwrap();
    rows.retain(|v| root.join(v["path"].as_str().unwrap()).starts_with(&root));
    for row in rows {
        row["path"] = root
            .join(row["path"].as_str().unwrap())
            .strip_prefix(&root)
            .unwrap()
            .to_str()
            .unwrap()
            .into();
    }
    for input in f.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
        .as_array_mut()
        .unwrap()
    {
        let path = root.join(input["location"].as_str().unwrap());
        input["location"] = path.strip_prefix(&root).unwrap().to_str().unwrap().into();
    }
    f.values.get_mut(PROJECTS).unwrap()["projects"][1]["actions"]["execute"]["argv"][2] =
        ".".into();
    f.save();
}

#[test]
fn portable_policy_bytes_work_at_different_checkout_depths() {
    let parent = tempfile::tempdir().unwrap();
    let deeper = parent.path().join("nested directory/another level");
    fs::create_dir_all(&deeper).unwrap();
    let mut shallow = Fixture::new_in(Some(parent.path()));
    let mut deep = Fixture::new_in(Some(&deeper));
    portable(&mut shallow);
    portable(&mut deep);
    assert_eq!(
        shallow.contract, deep.contract,
        "moving must not rewrite policy"
    );
    assert_eq!(
        shallow.values[CONFIG]["environment"]["inputs"],
        deep.values[CONFIG]["environment"]["inputs"]
    );
    for f in [&shallow, &deep] {
        let (code, report, stderr) = f.call();
        assert_eq!(code, 0, "{report} {stderr}");
        assert_eq!(report["operation"]["exit_code"], 0);
        assert_eq!(
            fs::read(f.root().join(".chrono-harness/state/test-ran")).unwrap(),
            b"ran"
        );
        assert_eq!(report["input_closure_complete"], false);
        assert!(
            !report["configuration"]["ancestor_absences"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn ancestor_absence_is_enforced_for_both_names_even_if_registered() {
    for name in ["config", "config.toml"] {
        for register in [false, true] {
            let parent = tempfile::tempdir().unwrap();
            let mut f = Fixture::new_in(Some(parent.path()));
            portable(&mut f);
            let path = fs::canonicalize(parent.path())
                .unwrap()
                .join(".cargo")
                .join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            if register {
                f.configuration("parent.config", &path, "[env]\nCHRONO_AMBIENT='ancestor'\n");
                f.save();
            } else {
                fs::write(path, "[env]\nCHRONO_AMBIENT='ancestor'\n").unwrap();
            }
            rejected(&f, "ancestor configuration must be absent");
        }
    }
}

#[test]
fn v3_inventory_keeps_explicit_ancestor_inputs_and_relative_home() {
    let parent = tempfile::tempdir().unwrap();
    let mut f = Fixture::new_in(Some(parent.path()));
    f.contract["schema"] = "chrono-cargo-inputs/v3".into();
    f.contract["configuration_ancestors"] = "inventory".into();
    f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CARGO_HOME"] =
        ".chrono-harness/state/cargo-home".into();
    let path = fs::canonicalize(parent.path())
        .unwrap()
        .join(".cargo/config.toml");
    f.configuration("parent.config", &path, "[env]\nCHRONO_AMBIENT='explicit'\n");
    consumer(&f, "explicit");
    f.save();
    accepted(&f, "explicit");
}

#[test]
fn relative_cargo_home_configuration_reaches_the_real_consumer() {
    let mut f = Fixture::new();
    portable(&mut f);
    let path = f
        .root()
        .join(".chrono-harness/state/cargo-home/config.toml");
    f.configuration(
        "home.config",
        &path,
        "[env]\nCHRONO_AMBIENT='relative-home'\n",
    );
    consumer(&f, "relative-home");
    f.save();
    accepted(&f, "relative-home");
}

#[test]
fn v3_requires_a_known_explicit_ancestor_policy_and_keeps_v2_semantics() {
    for case in ["missing", "null", "unknown", "v2-policy", "overlap"] {
        let mut f = Fixture::new();
        portable(&mut f);
        match case {
            "missing" => {
                f.contract
                    .as_object_mut()
                    .unwrap()
                    .remove("configuration_ancestors");
            }
            "null" => f.contract["configuration_ancestors"] = serde_json::Value::Null,
            "unknown" => f.contract["configuration_ancestors"] = "discover".into(),
            "v2-policy" => f.contract["schema"] = "chrono-cargo-inputs/v2".into(),
            "overlap" => f.configuration_absent(&f.root().parent().unwrap().join(".cargo/config")),
            _ => unreachable!(),
        }
        f.save();
        rejected(
            &f,
            if case == "unknown" {
                "unknown variant"
            } else if case == "overlap" {
                "overlaps"
            } else {
                "contract schema/ancestor policy"
            },
        );
    }
}

#[test]
fn ancestor_absence_is_rechecked_after_the_actual_consumer() {
    let parent = tempfile::tempdir().unwrap();
    let mut f = Fixture::new_in(Some(parent.path()));
    portable(&mut f);
    let path = fs::canonicalize(parent.path())
        .unwrap()
        .join(".cargo/config.toml");
    f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CHRONO_MUTATE"] = json!(path);
    write(
        &f.root(),
        "t/src/lib.rs",
        "#[test] fn mutate_ancestor() { let p=std::path::PathBuf::from(std::env::var(\"CHRONO_MUTATE\").unwrap()); std::fs::create_dir_all(p.parent().unwrap()).unwrap(); std::fs::write(p,b\"[env]\\nCHRONO_AMBIENT='late'\\n\").unwrap(); }\n",
    );
    f.save();
    let (code, report, stderr) = f.call();
    assert_ne!(code, 0, "{report} {stderr}");
    assert_eq!(report["operation"]["exit_code"], 0, "{report}");
    assert!(path.is_file());
    assert!(
        report["error"]
            .as_str()
            .unwrap()
            .contains("ancestor configuration presence changed"),
        "{report}"
    );
}

#[cfg(unix)]
#[test]
fn ancestor_absence_and_relative_home_reject_symlink_locations() {
    for home in [false, true] {
        let parent = tempfile::tempdir().unwrap();
        let mut f = Fixture::new_in(Some(parent.path()));
        portable(&mut f);
        let target = parent.path().join("empty target");
        fs::create_dir_all(&target).unwrap();
        let link = if home {
            f.root().join("home-link")
        } else {
            parent.path().join(".cargo")
        };
        std::os::unix::fs::symlink(&target, &link).unwrap();
        if home {
            f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CARGO_HOME"] =
                "home-link".into();
            f.save();
        }
        rejected(&f, "configuration symlink");
    }
}

#[test]
fn portable_policy_runs_and_rejects_through_both_profiles_without_selecting_docs() {
    for consumer in [Consumer::Full, Consumer::Scoped] {
        for scenario in ["source", "ancestor", "docs"] {
            let parent = tempfile::tempdir().unwrap();
            let mut f = Fixture::new_in(Some(parent.path()));
            portable(&mut f);
            if scenario != "source" {
                write(
                    parent.path(),
                    ".cargo/config.toml",
                    "[env]\nCHRONO_AMBIENT='unexpected'\n",
                );
            }
            let (code, report) =
                f.committed_check(if scenario == "docs" { "docs" } else { "source" }, consumer);
            let rows = if consumer == Consumer::Full {
                &report["tests"]["executed"]
            } else {
                &report["response"]["evidence"]["executed"]
            };
            let rows = rows.as_array().unwrap_or_else(|| panic!("{report}"));
            if scenario == "docs" {
                assert_eq!(code, 0, "{report}");
                assert!(rows.is_empty(), "{report}");
            } else {
                assert_eq!(rows.len(), 1, "{report}");
                let process = if consumer == Consumer::Full {
                    &rows[0]["receipt"]["process"]
                } else {
                    &rows[0]["process"]
                };
                let bytes: Vec<u8> =
                    serde_json::from_value(process["stdout_bytes"].clone()).unwrap();
                let actual: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                if scenario == "source" {
                    assert_eq!(code, 0, "{report}");
                    assert_eq!(actual["operation"]["exit_code"], 0, "{actual}");
                    assert!(
                        !actual["configuration"]["ancestor_absences"]
                            .as_array()
                            .unwrap()
                            .is_empty()
                    );
                } else {
                    assert_ne!(code, 0, "{report}");
                    assert!(
                        actual["metadata"].is_null() && actual["operation"].is_null(),
                        "{actual}"
                    );
                    assert!(
                        actual["error"]
                            .as_str()
                            .unwrap()
                            .contains("ancestor configuration must be absent"),
                        "{actual}"
                    );
                }
            }
            assert_eq!(
                f.root().join(".chrono-harness/state/test-ran").exists(),
                scenario == "source"
            );
        }
    }
}

fn consumer(f: &Fixture, expected: &str) {
    write(
        &f.root(),
        "t/src/lib.rs",
        &format!(
            "#[test] fn configured_environment() {{ assert_eq!(p::value(),43); let value=std::env::var(\"CHRONO_AMBIENT\").unwrap(); assert_eq!(value,{expected:?}); std::fs::write(std::env::var(\"CHRONO_TEST_EFFECT\").unwrap(), value).unwrap(); }}\n"
        ),
    );
}
fn rejected(f: &Fixture, message: &str) {
    let (code, report, stderr) = f.call();
    assert_ne!(code, 0, "{report} {stderr}");
    assert!(
        report["error"].as_str().unwrap().contains(message),
        "{report}"
    );
    assert!(report["metadata"].is_null() && report["operation"].is_null());
    assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
}
fn accepted(f: &Fixture, expected: &str) {
    let (code, report, stderr) = f.call();
    assert_eq!(code, 0, "{report} {stderr}");
    assert_eq!(report["metadata"]["exit_code"], 0);
    assert_eq!(report["operation"]["exit_code"], 0);
    assert!(
        report["configuration"]["files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v["input"].is_null() && v["sha256"].is_null())
    );
    assert_eq!(report["input_closure_complete"], false);
    assert_eq!(
        fs::read_to_string(f.root().join(".chrono-harness/state/test-ran")).unwrap(),
        expected
    );
}
#[test]
fn unregistered_cargo_home_configuration_blocks_before_metadata() {
    let f = Fixture::new();
    let path = f
        .root()
        .join(".chrono-harness/state/cargo-home/config.toml");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "[env]\nCHRONO_AMBIENT='unregistered'\n").unwrap();
    let (code, report, stderr) = f.call();
    assert_ne!(
        code, 0,
        "ambient file must be explicitly registered: {report} {stderr}"
    );
    assert!(report["metadata"].is_null() && report["operation"].is_null());
    assert!(!f.root().join(".chrono-harness/state/test-ran").exists());
}
#[test]
fn registered_cargo_home_configuration_reaches_the_real_consumer() {
    let mut f = Fixture::new();
    let path = f
        .root()
        .join(".chrono-harness/state/cargo-home/config.toml");
    f.configuration("home.config", &path, "[env]\nCHRONO_AMBIENT='home-value'\n");
    consumer(&f, "home-value");
    f.save();
    accepted(&f, "home-value");
}
#[test]
fn ancestor_configuration_and_legacy_precedence_match_cargo() {
    let parent = tempfile::tempdir().unwrap();
    let mut f = Fixture::new_in(Some(parent.path()));
    let ancestor = fs::canonicalize(parent.path())
        .unwrap()
        .join(".cargo/config.toml");
    fs::create_dir_all(ancestor.parent().unwrap()).unwrap();
    fs::write(&ancestor, "[env]\nCHRONO_AMBIENT='ancestor'\n").unwrap();
    rejected(&f, "presence differs");
    f.configuration(
        "parent.config",
        &ancestor,
        "[env]\nCHRONO_AMBIENT='ancestor'\n",
    );
    consumer(&f, "ancestor");
    f.save();
    accepted(&f, "ancestor");
    fs::remove_file(f.root().join(".chrono-harness/state/test-ran")).unwrap();
    f.configuration(
        "root.legacy",
        &f.root().join(".cargo/config"),
        "[env]\nCHRONO_AMBIENT='legacy'\n",
    );
    // Cargo ignores the shadowed modern file, including its invalid TOML.
    f.configuration(
        "root.modern",
        &f.root().join(".cargo/config.toml"),
        "this is not TOML\n",
    );
    consumer(&f, "legacy");
    f.save();
    accepted(&f, "legacy");
}
#[test]
fn recursive_includes_keep_cargo_order_and_explicit_optional_absence() {
    let mut f = Fixture::new();
    let base = f.root().join(".chrono-harness/cargo/config.toml");
    let old = fs::read_to_string(&base).unwrap();
    f.configuration("cargo.config",&base,&format!("include=['first.toml', {{path='missing.toml', optional=true}}, 'nested/second.toml']\n{old}"));
    f.configuration(
        "first",
        &base.with_file_name("first.toml"),
        "[env]\nCHRONO_AMBIENT='first'\n",
    );
    f.configuration(
        "second",
        &base.parent().unwrap().join("nested/second.toml"),
        "include=['../leaf.toml']\n",
    );
    f.configuration(
        "leaf",
        &base.with_file_name("leaf.toml"),
        "[env]\nCHRONO_AMBIENT='second'\n",
    );
    f.configuration_absent(&base.with_file_name("missing.toml"));
    consumer(&f, "second");
    f.save();
    accepted(&f, "second");
}
#[test]
fn configuration_inventory_requires_edges_paths_states_and_explicit_cargo_home() {
    for case in [
        "missing-probe",
        "duplicate",
        "unknown-input",
        "disconnected",
        "wrong-path",
        "extra",
        "missing-state",
        "no-home",
        "relative-home",
        "v1",
    ] {
        let mut f = Fixture::new();
        let path = f
            .root()
            .join(".chrono-harness/state/cargo-home/config.toml");
        f.configuration("home.config", &path, "[env]\nCHRONO_AMBIENT='home'\n");
        match case {
            "missing-probe" => {
                f.contract["configuration_files"]
                    .as_array_mut()
                    .unwrap()
                    .remove(1);
            }
            "duplicate" => {
                let row = f.contract["configuration_files"][0].clone();
                f.contract["configuration_files"]
                    .as_array_mut()
                    .unwrap()
                    .push(row);
            }
            "unknown-input" => {
                let row = f.contract["configuration_files"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|v| v["input"] == "home.config")
                    .unwrap();
                row["input"] = "unknown.config".into();
            }
            "disconnected" => {
                f.values.get_mut(FM).unwrap()["project_edges"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|v| v["from"] != "input:home.config");
            }
            "wrong-path" => {
                let other = f.root().join(".chrono-harness/state/other.toml");
                fs::copy(&path, &other).unwrap();
                let row = f.values.get_mut(CONFIG).unwrap()["environment"]["inputs"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|v| v["id"] == "home.config")
                    .unwrap();
                row["location"] = json!(other);
            }
            "extra" => {
                f.configuration_absent(&f.root().join("unrelated.toml"));
            }
            "missing-state" => {
                f.contract["configuration_files"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("input");
            }
            "no-home" => {
                f.values.get_mut(CONFIG).unwrap()["environment"]["values"]
                    .as_object_mut()
                    .unwrap()
                    .remove("CARGO_HOME");
            }
            "relative-home" => {
                f.values.get_mut(CONFIG).unwrap()["environment"]["values"]["CARGO_HOME"] =
                    "relative-home".into();
            }
            _ => {
                f.contract["schema"] = "chrono-cargo-inputs/v1".into();
            }
        }
        f.save();
        rejected(
            &f,
            match case {
                "missing-probe" => "missing configuration lookup",
                "duplicate" => "duplicate configuration",
                "unknown-input" | "disconnected" => "uniquely registered/connected",
                "wrong-path" => "location differs",
                "extra" => "outside lookup",
                "missing-state" => "missing field",
                "no-home" | "relative-home" => "CARGO_HOME",
                _ => "v2",
            },
        );
    }
}
#[test]
fn undeclared_required_and_cyclic_includes_fail_before_cargo() {
    for case in [
        "undeclared",
        "required-absent",
        "cycle",
        "optional-undeclared",
    ] {
        let mut f = Fixture::new();
        let base = f.root().join(".chrono-harness/cargo/config.toml");
        let old = fs::read_to_string(&base).unwrap();
        let include = if case == "optional-undeclared" {
            "[{path='child.toml',optional=true}]"
        } else {
            "['child.toml']"
        };
        f.configuration("cargo.config", &base, &format!("include={include}\n{old}"));
        match case {
            "required-absent" => f.configuration_absent(&base.with_file_name("child.toml")),
            "cycle" => f.configuration(
                "child",
                &base.with_file_name("child.toml"),
                "include=['config.toml']\n",
            ),
            _ => {}
        }
        f.save();
        rejected(
            &f,
            match case {
                "required-absent" => "required configuration include",
                "cycle" => "include cycle",
                _ => "unregistered configuration include",
            },
        );
    }
}
#[test]
fn successful_consumer_cannot_leave_new_changed_or_missing_configuration() {
    for change in ["create", "change", "remove"] {
        let mut f = Fixture::new();
        let path = f
            .root()
            .join(".chrono-harness/state/cargo-home/config.toml");
        if change != "create" {
            f.configuration("home.config", &path, "[env]\nCHRONO_AMBIENT='before'\n");
        }
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let action = if change == "remove" {
            format!(
                "std::fs::remove_file({:?}).unwrap();",
                path.to_str().unwrap()
            )
        } else {
            format!(
                "std::fs::write({:?},\"[env]\\nCHRONO_AMBIENT='after'\\n\").unwrap();",
                path.to_str().unwrap()
            )
        };
        write(
            &f.root(),
            "t/src/lib.rs",
            &format!("#[test] fn mutate_config() {{ assert_eq!(p::value(),43); {action} }}\n"),
        );
        f.save();
        let (code, report, stderr) = f.call();
        assert_ne!(code, 0, "{report} {stderr}");
        assert_eq!(report["metadata"]["exit_code"], 0);
        assert_eq!(report["operation"]["exit_code"], 0);
        assert!(
            report["error"]
                .as_str()
                .unwrap()
                .contains(if change == "change" {
                    "input changed during guarded operation"
                } else {
                    "presence changed during guarded operation"
                }),
            "{report}"
        );
    }
}

#[cfg(unix)]
#[test]
fn configuration_links_including_links_before_parent_components_are_rejected() {
    use std::os::unix::fs::symlink;
    for case in ["file", "dangling", "directory", "argument-parent"] {
        let mut f = Fixture::new();
        let path = f
            .root()
            .join(".chrono-harness/state/cargo-home/config.toml");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        if case == "argument-parent" {
            let alias = f.root().join("alias");
            symlink(f.root().join(".chrono-harness/cargo"), &alias).unwrap();
            let argument = format!("{}/../.chrono-harness/cargo/config.toml", alias.display());
            *f.contract["metadata"]["argv"]
                .as_array_mut()
                .unwrap()
                .last_mut()
                .unwrap() = argument.clone().into();
            *f.contract["operations"]["test.t"]["argv"]
                .as_array_mut()
                .unwrap()
                .last_mut()
                .unwrap() = argument.into();
        } else if case == "directory" {
            let home = path.parent().unwrap();
            fs::rename(home, home.with_file_name("original-cargo-home")).unwrap();
            symlink(f.root().join(".chrono-harness/cargo"), home).unwrap();
        } else {
            let target = f.root().join(".chrono-harness/cargo/link-target.toml");
            if case == "file" {
                fs::write(&target, "[env]\nCHRONO_AMBIENT='link'\n").unwrap();
            }
            symlink(target, &path).unwrap();
        }
        f.save();
        rejected(&f, "configuration symlink");
    }
}
