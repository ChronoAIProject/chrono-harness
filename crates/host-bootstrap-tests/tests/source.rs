#[path = "support/construction.rs"]
mod support;
use chrono_host_bootstrap::source::{observe, source_identity};
use serde_json::{Value, json};
use std::fs;
use support::Fixture;

fn value(source: impl serde::Serialize) -> Value {
    serde_json::to_value(source).unwrap()
}

#[test]
fn source_observation_keeps_mocked_clean_dirty_changed_and_unavailable_states() {
    let fixture = Fixture::new();
    let root = &fixture.root;
    let environment = &fixture.environment;
    let missing = observe(root, environment);
    assert!(missing.error.unwrap().contains("mock source unavailable"));
    assert_eq!(
        value(source_identity(
            observe(root, environment),
            observe(root, environment)
        ))["state"],
        "unavailable"
    );

    let commit = "a".repeat(40);
    let tree = "b".repeat(40);
    fixture.source(&commit, &tree, false);
    let clean = value(source_identity(
        observe(root, environment),
        observe(root, environment),
    ));
    assert_eq!(clean["state"], "clean");
    assert_eq!(clean["commit"], commit);
    assert_eq!(clean["tree"], tree);
    assert_eq!(clean["before"]["dirty"], false);
    assert_eq!(clean["before"]["error"], Value::Null);

    let before = observe(root, environment);
    fixture.dirty(true);
    let changed = value(source_identity(before, observe(root, environment)));
    assert_eq!(changed["state"], "changed");
    assert_eq!(changed["commit"], Value::Null);
    let dirty = value(source_identity(
        observe(root, environment),
        observe(root, environment),
    ));
    assert_eq!(dirty["state"], "dirty");
    assert_eq!(dirty["tree"], Value::Null);
    assert_eq!(dirty["before"]["commit"], commit);

    fixture.source(&commit, &tree, false);
    fixture.git_query(
        &["rev-parse", "--verify", "HEAD"],
        format!("{}\n", "c".repeat(40)),
    );
    let changed = observe(root, environment);
    assert_eq!(changed.commit.as_deref(), Some(commit.as_str()));
    assert_eq!(changed.tree.as_deref(), Some(tree.as_str()));
    assert_eq!(changed.dirty, Some(false));
    assert!(changed.error.unwrap().contains("HEAD changed"));

    let nested = root.join("nested");
    fs::create_dir(&nested).unwrap();
    fixture.mock_git(
        &["--no-optional-locks", "--no-replace-objects", "rev-parse", "--show-toplevel", "--verify", "HEAD"],
        json!({"cwd":nested,"exit_code":0,"stdout":format!("{}\n{commit}\n", root.display()).into_bytes()}),
    );
    assert!(
        observe(&nested, environment)
            .error
            .unwrap()
            .contains("not the Git source root")
    );
}

#[test]
fn source_git_boundary_clears_redirection_and_keeps_original_failure_after_partial_input() {
    let mut fixture = Fixture::new();
    let commit = "a".repeat(40);
    fixture.source(&commit, &"b".repeat(40), false);
    for key in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE"] {
        fixture
            .environment
            .insert(key.into(), "unrelated repository".into());
    }
    let path = fixture.tools.join("git.json");
    let mut specification: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    for response in specification.as_object_mut().unwrap().values_mut() {
        response["environment"] =
            json!({"GIT_DIR":null,"GIT_WORK_TREE":null,"GIT_INDEX_FILE":null});
    }
    fs::write(&path, serde_json::to_vec(&specification).unwrap()).unwrap();
    let observed = observe(&fixture.root, &fixture.environment);
    assert_eq!(observed.error, None);
    fixture.mock_git(
        &["--no-optional-locks", "--no-replace-objects", "rev-parse", "--verify", &format!("{commit}^{{tree}}")],
        json!({"exit_code":17,"stderr":b"original mock Git failure\n".to_vec(),"environment":{"GIT_DIR":null,"GIT_WORK_TREE":null,"GIT_INDEX_FILE":null}}),
    );
    let observed = observe(&fixture.root, &fixture.environment);
    assert_eq!(observed.commit.as_deref(), Some(commit.as_str()));
    assert_eq!(observed.tree, None);
    assert_eq!(observed.dirty, None);
    let error = observed.error.unwrap();
    assert!(error.contains("17"), "{error}");
    assert!(error.contains("original mock Git failure"), "{error}");
    let calls: Value =
        serde_json::from_slice(&fs::read(fixture.tools.join("git-calls.json")).unwrap()).unwrap();
    assert_eq!(calls.as_array().unwrap().len(), 6);
}
