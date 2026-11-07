use chrono_host_bootstrap::{
    bindings::Environment,
    source::{observe, source_identity},
};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE");
    chrono_harness::process_fds::forward_command(&mut command, &[]).unwrap();
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn value(source: impl serde::Serialize) -> Value {
    serde_json::to_value(source).unwrap()
}

#[test]
fn source_observation_retains_actual_clean_dirty_changed_and_unavailable_states() {
    let temporary = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temporary.path()).unwrap();
    let environment: Environment = std::env::vars_os().collect();
    let missing = observe(&root, &environment);
    assert!(missing.error.is_some());
    assert_eq!(
        value(source_identity(missing, observe(&root, &environment)))["state"],
        "unavailable"
    );
    git(&root, &["init", "--quiet"]);
    fs::write(root.join("product"), b"first\n").unwrap();
    git(&root, &["add", "product"]);
    git(
        &root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ],
    );
    let clean = value(source_identity(
        observe(&root, &environment),
        observe(&root, &environment),
    ));
    assert_eq!(clean["state"], "clean");
    assert_eq!(clean["commit"], clean["before"]["commit"]);
    assert_eq!(clean["tree"], clean["before"]["tree"]);
    assert_eq!(clean["before"]["dirty"], false);
    assert_eq!(clean["before"]["error"], Value::Null);
    let before = observe(&root, &environment);
    fs::write(root.join("product"), b"second\n").unwrap();
    let changed = value(source_identity(before, observe(&root, &environment)));
    assert_eq!(changed["state"], "changed");
    assert_eq!(changed["commit"], Value::Null);
    let dirty = value(source_identity(
        observe(&root, &environment),
        observe(&root, &environment),
    ));
    assert_eq!(dirty["state"], "dirty");
    assert_eq!(dirty["tree"], Value::Null);
    assert_eq!(dirty["before"]["commit"], clean["commit"]);
    let child = root.join("nested");
    fs::create_dir(&child).unwrap();
    let wrong = observe(&child, &environment);
    assert!(wrong.error.unwrap().contains("not the Git source root"));
    let original: chrono_host_bootstrap::source::SourceObservation =
        serde_json::from_value(clean["before"].clone()).unwrap();
    let mut renamed = clean["before"].clone();
    renamed["tree"] = json!("another tree");
    assert_eq!(
        value(source_identity(
            original,
            serde_json::from_value(renamed).unwrap()
        ))["state"],
        "changed"
    );
}
