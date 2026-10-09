#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
use chrono_judge_cargo::{Policy, check};
use serde_json::json;
use std::{collections::BTreeSet, fs};
use support::*;

fn fixture() -> (tempfile::TempDir, Values, Policy) {
    let dir = tempfile::tempdir().unwrap();
    let mut v = values();
    for f in v.get_mut(FM).unwrap()["files"].as_array_mut().unwrap() {
        let path = f["path"].as_str().unwrap();
        if path.starts_with("p/") {
            f["owner"] = json!("p");
        } else if path.starts_with("t/") {
            f["owner"] = json!("t");
        }
    }
    for f in v[FM]["files"].as_array().unwrap() {
        let p = dir.path().join(f["path"].as_str().unwrap());
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, "fixture").unwrap();
    }
    for id in ["p", "t"] {
        fs::write(
            dir.path().join(format!("{id}/Cargo.toml")),
            format!("[package]\nname='{id}'\nversion='0.1.0'\n"),
        )
        .unwrap();
        v.get_mut(CONFIG).unwrap()["artifacts"]
            .as_array_mut()
            .unwrap()
            .push(json!({"path":format!("out/{id}/"),"owner":id,"kind":"output","tracked":false}));
    }
    let policy=serde_json::from_value(json!({"schema":"chrono-cargo-projects/v1","projects":[{"project":"p","manifest":"p/Cargo.toml","lockfile":"p/Cargo.lock","output":"out/p/","ancestor_manifests":[]},{"project":"t","manifest":"t/Cargo.toml","lockfile":"t/Cargo.lock","output":"out/t/","ancestor_manifests":[]}]})).unwrap();
    (dir, v, policy)
}
fn selected() -> BTreeSet<String> {
    BTreeSet::from(["p".into(), "t".into()])
}
#[test]
fn custom_declared_outputs_and_absent_generic_manifest_fields_are_accepted() {
    let (d, mut v, p) = fixture();
    for row in v.get_mut(PROJECTS).unwrap()["projects"]
        .as_array_mut()
        .unwrap()
    {
        for key in ["manifest", "lockfile", "root"] {
            row.as_object_mut().unwrap().remove(key);
        }
    }
    assert_eq!(
        check(d.path(), &load(&v), &p, &selected()).unwrap(),
        vec!["p", "t"]
    );
}
#[test]
fn missing_output_or_input_ownership_fails_only_selected_projects() {
    for output in [false, true] {
        let (d, mut v, p) = fixture();
        if output {
            v.get_mut(CONFIG).unwrap()["artifacts"]
                .as_array_mut()
                .unwrap()
                .retain(|a| a["owner"] != "p");
        } else {
            v.get_mut(FM).unwrap()["files"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|f| f["path"] == "p/Cargo.lock")
                .unwrap()["owner"] = json!("t");
        }
        assert!(
            check(d.path(), &load(&v), &p, &selected())
                .unwrap_err()
                .contains("E_TEST_PAIR")
        );
        assert!(
            check(d.path(), &load(&v), &p, &BTreeSet::new())
                .unwrap()
                .is_empty()
        );
    }
}
#[test]
fn explicit_path_dependencies_check_all_cargo_kinds_without_creating_edges() {
    for table in [
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "target.'cfg(unix)'.dependencies",
    ] {
        let (d, mut v, p) = fixture();
        fs::write(d.path().join("t/Cargo.toml"),format!("[package]\nname='t'\nversion='0.1.0'\n[{table}]\nrenamed={{package='p',path='../p'}}\n")).unwrap();
        assert!(
            check(d.path(), &load(&v), &p, &selected())
                .unwrap_err()
                .contains("E_DANGLING_EDGE")
        );
        v.get_mut(FM).unwrap()["project_edges"]
            .as_array_mut()
            .unwrap()
            .push(edge("project:p", "compile", "project:t"));
        let before = v.clone();
        check(d.path(), &load(&v), &p, &selected()).unwrap();
        assert_eq!(v, before);
    }
}
#[test]
fn workspace_unregistered_dependency_and_external_closure_are_not_silently_accepted() {
    for (body, code) in [
        ("[workspace]\nmembers=[]\n", "E_TEST_PAIR"),
        ("[package]\nworkspace='..'\n", "E_TEST_PAIR"),
        ("[dependencies]\nx={workspace=true}\n", "E_TEST_PAIR"),
        (
            "[dependencies]\nx={path='../missing'}\n",
            "E_INPUT_UNDECLARED",
        ),
        ("[dependencies]\nx='1'\n", "E_INPUT_UNDECLARED"),
        (
            "[dependencies]\nx={git='https://example.invalid/x'}\n",
            "E_INPUT_UNDECLARED",
        ),
    ] {
        let (d, v, p) = fixture();
        fs::write(d.path().join("p/Cargo.toml"), body).unwrap();
        assert!(
            check(d.path(), &load(&v), &p, &selected())
                .unwrap_err()
                .contains(code)
        );
    }
}
#[test]
fn ancestor_inventory_requires_explicit_registration_and_does_not_rejudge_unselected_history() {
    let (d, mut v, mut p) = fixture();
    fs::write(
        d.path().join("Cargo.toml"),
        "[workspace]\nmembers=['p','t']\n",
    )
    .unwrap();
    v.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(file("Cargo.toml", json!([])));
    assert!(
        check(d.path(), &load(&v), &p, &selected())
            .unwrap_err()
            .contains("ancestor inventory")
    );
    for row in &mut p.projects {
        row.ancestor_manifests.push("Cargo.toml".into());
    }
    assert!(
        check(d.path(), &load(&v), &p, &selected())
            .unwrap_err()
            .contains("registered ancestor workspace")
    );
    assert!(
        check(d.path(), &load(&v), &p, &BTreeSet::new())
            .unwrap()
            .is_empty()
    );
}

#[test]
fn standalone_registered_prerequisite_consumes_explicit_selection_and_real_files() {
    use std::{path::Path, process::Command};
    let (d, mut v, p) = fixture();
    let policy = ".chrono-harness/cargo/projects.json";
    v.get_mut(FM).unwrap()["files"]
        .as_array_mut()
        .unwrap()
        .push(file(policy, json!([])));
    v.insert(policy.into(), serde_json::to_value(p).unwrap());
    write_values(d.path(), &v);
    let tool = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../judge-cargo/target/debug/chrono-judge-cargo");
    let call = |selected: &str| {
        Command::new(&tool)
            .args([
                "check",
                "--host-root",
                d.path().to_str().unwrap(),
                "--config",
                CONFIG,
                "--policy",
                policy,
                "--project",
                selected,
            ])
            .current_dir("/")
            .output()
            .unwrap()
    };
    let good = call("p");
    assert!(
        good.status.success(),
        "{}",
        String::from_utf8_lossy(&good.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&good.stdout).unwrap();
    assert_eq!(report["checked"], json!(["p"]));
    assert!(!call("unregistered").status.success());
    fs::write(d.path().join("p/Cargo.toml"), "[workspace]\n").unwrap();
    let bad = call("p");
    assert_eq!(bad.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&bad.stderr).contains("Cargo workspace aggregation"));
    assert!(call("t").status.success());
}
