#[path = "support/cargo.rs"]
mod cargo;
use cargo::*;
use serde_json::json;
use std::fs;

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
